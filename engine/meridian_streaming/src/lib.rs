//! Deterministic cell-residency state and request scheduling.
//!
//! IO, decompression, GPU upload, and activation are deliberately separate
//! follow-up stages. This crate owns the bounded scheduling decisions between
//! those stages.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;

use meridian_assets::{
    AssetDecoder, AssetLoadError, AssetLoadRequest, AssetLoadResult, PackReader,
};
use meridian_core::{OperationId, RuntimeEpoch, TraceId};
use meridian_tasks::{Task, TaskClass, TaskContext, TaskError, TaskPool};
pub use meridian_world::WorldCell;

/// Residency stages for one world cell, from metadata discovery to activation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellResidencyState {
    Unknown,
    MetadataOnly,
    CpuCompressed,
    CpuDecoded,
    GpuQueued,
    GpuResident,
    Active,
    EvictionCandidate,
}

impl CellResidencyState {
    #[must_use]
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Unknown, Self::MetadataOnly)
                | (Self::MetadataOnly, Self::CpuCompressed)
                | (
                    Self::CpuCompressed,
                    Self::CpuDecoded | Self::EvictionCandidate
                )
                | (Self::CpuDecoded, Self::GpuQueued | Self::EvictionCandidate)
                | (Self::GpuQueued | Self::EvictionCandidate, Self::GpuResident)
                | (Self::GpuResident, Self::Active | Self::EvictionCandidate)
                | (Self::Active, Self::EvictionCandidate)
                | (Self::EvictionCandidate, Self::Unknown)
        )
    }
}

/// Scheduler input for one requested cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellRequest {
    cell: WorldCell,
    priority: i32,
}

impl CellRequest {
    #[must_use]
    pub const fn new(cell: WorldCell, priority: i32) -> Self {
        Self { cell, priority }
    }

    #[must_use]
    pub const fn cell(self) -> WorldCell {
        self.cell
    }

    #[must_use]
    pub const fn priority(self) -> i32 {
        self.priority
    }
}

/// Observable state tracked for one requested or resident cell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellResidencyRecord {
    cell: WorldCell,
    state: CellResidencyState,
    priority: i32,
    last_requested_tick: u64,
    requested: bool,
    cancel_requested: bool,
}

impl CellResidencyRecord {
    #[must_use]
    pub const fn cell(self) -> WorldCell {
        self.cell
    }

    #[must_use]
    pub const fn state(self) -> CellResidencyState {
        self.state
    }

    #[must_use]
    pub const fn priority(self) -> i32 {
        self.priority
    }

    #[must_use]
    pub const fn last_requested_tick(self) -> u64 {
        self.last_requested_tick
    }

    #[must_use]
    pub const fn requested(self) -> bool {
        self.requested
    }

    #[must_use]
    pub const fn cancel_requested(self) -> bool {
        self.cancel_requested
    }
}

/// Errors from invalid streaming state operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingError {
    UnknownCell(WorldCell),
    InvalidTransition {
        cell: WorldCell,
        from: CellResidencyState,
        to: CellResidencyState,
    },
}

impl Display for StreamingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCell(cell) => write!(formatter, "unknown streaming cell: {cell:?}"),
            Self::InvalidTransition { cell, from, to } => write!(
                formatter,
                "invalid streaming transition for {cell:?}: {from:?} -> {to:?}"
            ),
        }
    }
}

impl Error for StreamingError {}

/// Bounded, deterministic scheduler for cell residency requests.
#[derive(Debug, Default)]
pub struct StreamingScheduler {
    records: BTreeMap<WorldCell, CellResidencyRecord>,
    pending: BTreeSet<WorldCell>,
}

/// One decoded cell activation operation waiting for a bounded main-thread slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationWork {
    cell: WorldCell,
    bytes: usize,
    priority: i32,
}

impl ActivationWork {
    #[must_use]
    pub const fn new(cell: WorldCell, bytes: usize, priority: i32) -> Self {
        Self {
            cell,
            bytes,
            priority,
        }
    }

    #[must_use]
    pub const fn cell(self) -> WorldCell {
        self.cell
    }

    #[must_use]
    pub const fn bytes(self) -> usize {
        self.bytes
    }

    #[must_use]
    pub const fn priority(self) -> i32 {
        self.priority
    }
}

/// Errors raised when an activation queue would exceed a hard budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationQueueError {
    DuplicateCell(WorldCell),
    ItemCapacity {
        limit: usize,
    },
    ByteCapacity {
        requested: usize,
        queued: usize,
        limit: usize,
    },
}

impl Display for ActivationQueueError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCell(cell) => write!(formatter, "activation already queued: {cell:?}"),
            Self::ItemCapacity { limit } => {
                write!(formatter, "activation item budget reached: {limit}")
            }
            Self::ByteCapacity {
                requested,
                queued,
                limit,
            } => write!(
                formatter,
                "activation byte budget exceeded: requested {requested}, queued {queued}, limit {limit}"
            ),
        }
    }
}

impl Error for ActivationQueueError {}

/// Deterministic activation queue bounded by item count and estimated bytes.
#[derive(Debug)]
pub struct ActivationQueue {
    max_items: usize,
    max_bytes: usize,
    queued_bytes: usize,
    entries: BTreeMap<WorldCell, ActivationWork>,
}

/// Errors raised before a per-cell background load can be submitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CellLoadSubmitError {
    DuplicateCell(WorldCell),
    PoolClosed,
}

impl Display for CellLoadSubmitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCell(cell) => write!(formatter, "cell load already pending: {cell:?}"),
            Self::PoolClosed => formatter.write_str("streaming worker pool is closed"),
        }
    }
}

impl Error for CellLoadSubmitError {}

/// Errors observed after a cell load has been submitted.
#[derive(Debug, Eq, PartialEq)]
pub enum CellLoadError {
    Asset(AssetLoadError),
    Task(TaskError),
}

impl Display for CellLoadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asset(error) => Display::fmt(error, formatter),
            Self::Task(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CellLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Asset(error) => Some(error),
            Self::Task(error) => Some(error),
        }
    }
}

/// The result of one completed background cell load.
#[derive(Debug, Eq, PartialEq)]
pub struct CellLoadCompletion {
    cell: WorldCell,
    priority: i32,
    task_id: u64,
    context: TaskContext,
    result: Result<AssetLoadResult, CellLoadError>,
}

impl CellLoadCompletion {
    #[must_use]
    pub const fn cell(&self) -> WorldCell {
        self.cell
    }

    #[must_use]
    pub const fn priority(&self) -> i32 {
        self.priority
    }

    #[must_use]
    pub const fn task_id(&self) -> u64 {
        self.task_id
    }

    #[must_use]
    pub const fn context(&self) -> TaskContext {
        self.context
    }

    /// Returns the completed asset result, transferring ownership to the caller.
    ///
    /// # Errors
    ///
    /// Returns an asset or task error when the background operation failed.
    pub fn into_result(self) -> Result<AssetLoadResult, CellLoadError> {
        self.result
    }
}

struct PendingCellLoad {
    priority: i32,
    context: TaskContext,
    cancellation: meridian_assets::CancellationToken,
    task: Task<Result<AssetLoadResult, AssetLoadError>>,
}

/// Bounded worker-backed coordinator for cell asset IO and decompression.
///
/// The coordinator owns no world or GPU state. Requests execute through the
/// existing [`AssetLoadRequest`] boundary on fixed workers, while the caller
/// polls completed results and decides when to enqueue activation work.
pub struct CellLoadCoordinator {
    pool: TaskPool,
    pending: BTreeMap<WorldCell, PendingCellLoad>,
}

impl CellLoadCoordinator {
    /// Starts a coordinator with exactly `worker_count` fixed workers.
    #[must_use]
    pub fn new(worker_count: NonZeroUsize) -> Self {
        Self {
            pool: TaskPool::new(worker_count),
            pending: BTreeMap::new(),
        }
    }

    /// Starts a coordinator using the host's reported parallelism.
    #[must_use]
    pub fn with_default_workers() -> Self {
        Self {
            pool: TaskPool::with_default_workers(),
            pending: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn worker_count(&self) -> NonZeroUsize {
        self.pool.worker_count()
    }

    /// Submits one stable-ID asset load for a world cell.
    ///
    /// # Errors
    ///
    /// Returns [`CellLoadSubmitError::DuplicateCell`] without replacing an
    /// existing job, or [`CellLoadSubmitError::PoolClosed`] if submission
    /// fails because the worker pool is shutting down.
    pub fn submit<R, D>(
        &mut self,
        cell: WorldCell,
        priority: i32,
        request: AssetLoadRequest,
        reader: R,
        decoder: D,
    ) -> Result<(), CellLoadSubmitError>
    where
        R: PackReader + Send + 'static,
        D: AssetDecoder + Send + 'static,
    {
        self.submit_correlated(
            cell,
            priority,
            TaskContext::new(
                TaskClass::Streaming,
                OperationId::default(),
                TraceId::default(),
                RuntimeEpoch::default(),
            ),
            request,
            reader,
            decoder,
        )
    }

    /// Submits one cell load carrying operation, trace, and lifecycle epoch.
    ///
    /// # Errors
    ///
    /// Returns duplicate/pool errors without changing existing work.
    pub fn submit_correlated<R, D>(
        &mut self,
        cell: WorldCell,
        priority: i32,
        context: TaskContext,
        request: AssetLoadRequest,
        reader: R,
        decoder: D,
    ) -> Result<(), CellLoadSubmitError>
    where
        R: PackReader + Send + 'static,
        D: AssetDecoder + Send + 'static,
    {
        if self.pending.contains_key(&cell) {
            return Err(CellLoadSubmitError::DuplicateCell(cell));
        }
        let cancellation = request.cancellation();
        let task = self
            .pool
            .submit_correlated(context, request.into_job(reader, decoder))
            .map_err(|_| CellLoadSubmitError::PoolClosed)?;
        self.pending.insert(
            cell,
            PendingCellLoad {
                priority,
                context,
                cancellation,
                task,
            },
        );
        Ok(())
    }

    /// Requests cancellation and removes a pending cell load.
    #[must_use]
    pub fn cancel(&mut self, cell: WorldCell) -> bool {
        let Some(pending) = self.pending.remove(&cell) else {
            return false;
        };
        pending.cancellation.cancel();
        true
    }

    #[must_use]
    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Polls pending jobs in deterministic priority/cell order.
    #[must_use]
    pub fn poll(&mut self) -> Option<CellLoadCompletion> {
        let mut cells = self.pending.keys().copied().collect::<Vec<_>>();
        cells.sort_unstable_by_key(|cell| {
            self.pending
                .get(cell)
                .map_or((i32::MAX, *cell), |pending| (-pending.priority, *cell))
        });
        let ready = cells.into_iter().find_map(|cell| {
            self.pending.get_mut(&cell).and_then(|pending| {
                pending.task.poll().map(|result| {
                    (
                        cell,
                        pending.priority,
                        pending.task.id(),
                        pending.context,
                        result,
                    )
                })
            })
        });
        let (cell, priority, task_id, context, result) = ready?;
        self.pending.remove(&cell);
        let result = match result {
            Ok(Ok(asset)) => Ok(asset),
            Ok(Err(error)) => Err(CellLoadError::Asset(error)),
            Err(error) => Err(CellLoadError::Task(error)),
        };
        Some(CellLoadCompletion {
            cell,
            priority,
            task_id,
            context,
            result,
        })
    }
}

impl ActivationQueue {
    #[must_use]
    pub fn new(max_items: usize, max_bytes: usize) -> Self {
        Self {
            max_items,
            max_bytes,
            queued_bytes: 0,
            entries: BTreeMap::new(),
        }
    }

    /// Adds one activation operation if both hard budgets permit it.
    ///
    /// # Errors
    ///
    /// Returns a capacity or duplicate-cell error without changing the queue.
    pub fn enqueue(&mut self, work: ActivationWork) -> Result<(), ActivationQueueError> {
        if self.entries.contains_key(&work.cell) {
            return Err(ActivationQueueError::DuplicateCell(work.cell));
        }
        if self.entries.len() >= self.max_items {
            return Err(ActivationQueueError::ItemCapacity {
                limit: self.max_items,
            });
        }
        if work.bytes > self.max_bytes.saturating_sub(self.queued_bytes) {
            return Err(ActivationQueueError::ByteCapacity {
                requested: work.bytes,
                queued: self.queued_bytes,
                limit: self.max_bytes,
            });
        }
        self.queued_bytes = self.queued_bytes.saturating_add(work.bytes);
        self.entries.insert(work.cell, work);
        Ok(())
    }

    /// Drains at most `max_items` while respecting the per-frame byte budget.
    /// Oversized lower-priority work is skipped so smaller work can proceed.
    #[must_use]
    pub fn drain_budget(&mut self, max_items: usize, max_bytes: usize) -> Vec<ActivationWork> {
        let mut candidates = self.entries.values().copied().collect::<Vec<_>>();
        candidates.sort_unstable_by_key(|work| (-work.priority, work.cell));

        let mut selected = Vec::new();
        let mut remaining_bytes = max_bytes;
        for work in candidates {
            if selected.len() >= max_items {
                break;
            }
            if work.bytes <= remaining_bytes {
                remaining_bytes -= work.bytes;
                selected.push(work);
            }
        }

        for work in &selected {
            self.entries.remove(&work.cell);
            self.queued_bytes = self.queued_bytes.saturating_sub(work.bytes);
        }
        selected
    }

    #[must_use]
    pub fn cancel(&mut self, cell: WorldCell) -> Option<ActivationWork> {
        let work = self.entries.remove(&cell)?;
        self.queued_bytes = self.queued_bytes.saturating_sub(work.bytes);
        Some(work)
    }

    #[must_use]
    pub const fn queued_bytes(&self) -> usize {
        self.queued_bytes
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl StreamingScheduler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests a cell and updates its priority without duplicating the queue entry.
    pub fn request(&mut self, request: CellRequest, tick: u64) {
        let record = self
            .records
            .entry(request.cell)
            .or_insert(CellResidencyRecord {
                cell: request.cell,
                state: CellResidencyState::Unknown,
                priority: request.priority,
                last_requested_tick: tick,
                requested: false,
                cancel_requested: false,
            });
        record.priority = request.priority;
        record.last_requested_tick = tick;
        record.requested = true;
        record.cancel_requested = false;
        self.pending.insert(request.cell);
    }

    /// Pops at most `limit` pending requests in descending priority order.
    #[must_use]
    pub fn pop_requests(&mut self, limit: usize) -> Vec<CellRequest> {
        let mut cells = self.pending.iter().copied().collect::<Vec<_>>();
        cells.sort_unstable_by_key(|cell| {
            self.records
                .get(cell)
                .map_or((i32::MAX, *cell), |record| (-record.priority, *cell))
        });
        cells.truncate(limit);

        cells
            .into_iter()
            .filter_map(|cell| {
                self.pending.remove(&cell);
                let record = self.records.get_mut(&cell)?;
                record.requested = false;
                Some(CellRequest::new(cell, record.priority))
            })
            .collect()
    }

    /// Advances one cell through the explicit residency state machine.
    ///
    /// # Errors
    ///
    /// Returns [`StreamingError::UnknownCell`] for an untracked cell or
    /// [`StreamingError::InvalidTransition`] when the requested edge is not
    /// part of the residency state machine.
    pub fn transition(
        &mut self,
        cell: WorldCell,
        next: CellResidencyState,
    ) -> Result<(), StreamingError> {
        let record = self
            .records
            .get_mut(&cell)
            .ok_or(StreamingError::UnknownCell(cell))?;
        if !record.state.can_transition_to(next) {
            return Err(StreamingError::InvalidTransition {
                cell,
                from: record.state,
                to: next,
            });
        }
        record.state = next;
        if next == CellResidencyState::Unknown {
            record.cancel_requested = false;
        }
        Ok(())
    }

    /// Requests cancellation of pending work for a known cell.
    ///
    /// # Errors
    ///
    /// Returns [`StreamingError::UnknownCell`] when the cell is not tracked.
    pub fn cancel(&mut self, cell: WorldCell) -> Result<(), StreamingError> {
        let record = self
            .records
            .get_mut(&cell)
            .ok_or(StreamingError::UnknownCell(cell))?;
        self.pending.remove(&cell);
        record.requested = false;
        record.cancel_requested = true;
        Ok(())
    }

    /// Returns a deterministic view of all tracked cells.
    pub fn records(&self) -> impl Iterator<Item = CellResidencyRecord> + '_ {
        self.records.values().copied()
    }

    #[must_use]
    pub fn get(&self, cell: WorldCell) -> Option<CellResidencyRecord> {
        self.records.get(&cell).copied()
    }

    /// Selects the lowest-priority resident/active cells for bounded eviction work.
    #[must_use]
    pub fn eviction_candidates(&self, limit: usize) -> Vec<WorldCell> {
        let mut candidates = self
            .records
            .values()
            .filter(|record| {
                matches!(
                    record.state,
                    CellResidencyState::GpuResident | CellResidencyState::Active
                )
            })
            .copied()
            .collect::<Vec<_>>();
        candidates.sort_unstable_by_key(|record| {
            (record.priority, record.last_requested_tick, record.cell)
        });
        candidates
            .into_iter()
            .take(limit)
            .map(|record| record.cell)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_assets::{
        AssetCompression, AssetId, AssetLoadError, AssetLoadRequest, CancellationToken,
        PackIndexEntry, UncompressedDecoder,
    };
    use std::sync::{Arc, Mutex};
    use std::thread::{self, ThreadId};

    fn cell(x: i64) -> WorldCell {
        WorldCell { x, y: 0, z: 0 }
    }

    struct TestReader {
        bytes: Vec<u8>,
        worker_thread: Arc<Mutex<Option<ThreadId>>>,
    }

    impl PackReader for TestReader {
        type Error = String;

        fn read_range(
            &mut self,
            offset: u64,
            length: u64,
            cancellation: &CancellationToken,
        ) -> Result<Vec<u8>, Self::Error> {
            if cancellation.is_cancelled() {
                return Err(String::from("cancelled"));
            }
            *self
                .worker_thread
                .lock()
                .expect("worker thread marker is not poisoned") = Some(thread::current().id());
            let start = usize::try_from(offset).map_err(|_| String::from("offset overflow"))?;
            let length = usize::try_from(length).map_err(|_| String::from("length overflow"))?;
            let end = start
                .checked_add(length)
                .ok_or_else(|| String::from("range overflow"))?;
            self.bytes
                .get(start..end)
                .map(ToOwned::to_owned)
                .ok_or_else(|| String::from("range outside test pack"))
        }
    }

    fn request(name: &str, token: CancellationToken) -> AssetLoadRequest {
        AssetLoadRequest::new(
            PackIndexEntry::new(
                AssetId::from_name(name),
                0,
                5,
                5,
                AssetCompression::None,
                "test-hash",
            ),
            token,
        )
    }

    #[test]
    fn requests_are_bounded_prioritized_and_not_duplicated() {
        let mut scheduler = StreamingScheduler::new();
        scheduler.request(CellRequest::new(cell(1), 10), 1);
        scheduler.request(CellRequest::new(cell(2), 30), 2);
        scheduler.request(CellRequest::new(cell(1), 40), 3);

        assert_eq!(
            scheduler.pop_requests(2),
            [CellRequest::new(cell(1), 40), CellRequest::new(cell(2), 30)]
        );
        assert!(scheduler.pop_requests(1).is_empty());
    }

    #[test]
    fn residency_transitions_are_explicit_and_invalid_edges_are_rejected() {
        let mut scheduler = StreamingScheduler::new();
        scheduler.request(CellRequest::new(cell(4), 1), 0);

        for state in [
            CellResidencyState::MetadataOnly,
            CellResidencyState::CpuCompressed,
            CellResidencyState::CpuDecoded,
            CellResidencyState::GpuQueued,
            CellResidencyState::GpuResident,
            CellResidencyState::Active,
            CellResidencyState::EvictionCandidate,
            CellResidencyState::Unknown,
        ] {
            scheduler
                .transition(cell(4), state)
                .expect("lifecycle transition is valid");
        }

        assert_eq!(
            scheduler.transition(cell(4), CellResidencyState::Active),
            Err(StreamingError::InvalidTransition {
                cell: cell(4),
                from: CellResidencyState::Unknown,
                to: CellResidencyState::Active,
            })
        );
    }

    #[test]
    fn cancellation_removes_pending_work_and_eviction_order_is_deterministic() {
        let mut scheduler = StreamingScheduler::new();
        for (x, priority, tick) in [(3, 5, 30), (1, 5, 10), (2, 9, 20)] {
            scheduler.request(CellRequest::new(cell(x), priority), tick);
            assert_eq!(
                scheduler.pop_requests(1),
                [CellRequest::new(cell(x), priority)]
            );
            scheduler
                .transition(cell(x), CellResidencyState::MetadataOnly)
                .expect("metadata transition");
            scheduler
                .transition(cell(x), CellResidencyState::CpuCompressed)
                .expect("compressed transition");
            scheduler
                .transition(cell(x), CellResidencyState::CpuDecoded)
                .expect("decoded transition");
            scheduler
                .transition(cell(x), CellResidencyState::GpuQueued)
                .expect("GPU queue transition");
            scheduler
                .transition(cell(x), CellResidencyState::GpuResident)
                .expect("resident transition");
        }
        scheduler.request(CellRequest::new(cell(3), 50), 31);
        scheduler.cancel(cell(3)).expect("known cell cancels");
        assert!(scheduler.pop_requests(10).is_empty());
        assert_eq!(scheduler.eviction_candidates(2), [cell(1), cell(2)]);
        assert!(scheduler.cancel(cell(99)).is_err());
    }

    #[test]
    fn activation_queue_enforces_item_and_byte_budgets() {
        let mut queue = ActivationQueue::new(2, 10);
        queue
            .enqueue(ActivationWork::new(cell(1), 6, 1))
            .expect("first activation fits");
        assert_eq!(
            queue.enqueue(ActivationWork::new(cell(1), 1, 2)),
            Err(ActivationQueueError::DuplicateCell(cell(1)))
        );
        assert_eq!(
            queue.enqueue(ActivationWork::new(cell(2), 5, 2)),
            Err(ActivationQueueError::ByteCapacity {
                requested: 5,
                queued: 6,
                limit: 10,
            })
        );
        queue
            .enqueue(ActivationWork::new(cell(2), 4, 2))
            .expect("second activation fits");
        assert_eq!(
            queue.enqueue(ActivationWork::new(cell(3), 0, 3)),
            Err(ActivationQueueError::ItemCapacity { limit: 2 })
        );
        assert_eq!(queue.queued_bytes(), 10);
    }

    #[test]
    fn activation_drain_is_priority_ordered_and_budgeted() {
        let mut queue = ActivationQueue::new(4, 32);
        queue
            .enqueue(ActivationWork::new(cell(1), 8, 10))
            .expect("activation fits");
        queue
            .enqueue(ActivationWork::new(cell(2), 4, 1))
            .expect("activation fits");
        queue
            .enqueue(ActivationWork::new(cell(3), 7, 8))
            .expect("activation fits");

        assert_eq!(
            queue.drain_budget(3, 12),
            [
                ActivationWork::new(cell(1), 8, 10),
                ActivationWork::new(cell(2), 4, 1)
            ]
        );
        assert_eq!(queue.queued_bytes(), 7);
        assert_eq!(
            queue.cancel(cell(3)),
            Some(ActivationWork::new(cell(3), 7, 8))
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn cell_loads_run_on_workers_and_return_pollable_results() {
        let main_thread = thread::current().id();
        let worker_thread = Arc::new(Mutex::new(None));
        let mut coordinator =
            CellLoadCoordinator::new(NonZeroUsize::new(1).expect("one worker is non-zero"));
        coordinator
            .submit(
                cell(1),
                10,
                request("cell-one", CancellationToken::new()),
                TestReader {
                    bytes: b"first".to_vec(),
                    worker_thread: Arc::clone(&worker_thread),
                },
                UncompressedDecoder,
            )
            .expect("cell load submits");
        assert_eq!(coordinator.pending_len(), 1);

        let completion = loop {
            if let Some(completion) = coordinator.poll() {
                break completion;
            }
            thread::yield_now();
        };
        assert_eq!(completion.cell(), cell(1));
        assert_eq!(completion.priority(), 10);
        assert_eq!(completion.task_id(), 0);
        let result = completion.into_result().expect("cell load succeeds");
        assert_eq!(result.bytes, b"first");
        assert_ne!(
            *worker_thread
                .lock()
                .expect("worker thread marker is not poisoned"),
            Some(main_thread)
        );
        assert_eq!(coordinator.pending_len(), 0);
    }

    #[test]
    fn correlated_cell_load_preserves_operation_trace_and_epoch() {
        let context = TaskContext::new(
            TaskClass::Streaming,
            OperationId::new(17),
            TraceId::new(23),
            RuntimeEpoch::new(4),
        );
        let mut coordinator =
            CellLoadCoordinator::new(NonZeroUsize::new(1).expect("one worker is non-zero"));
        coordinator
            .submit_correlated(
                cell(7),
                5,
                context,
                request("cell-seven", CancellationToken::new()),
                TestReader {
                    bytes: b"trace".to_vec(),
                    worker_thread: Arc::new(Mutex::new(None)),
                },
                UncompressedDecoder,
            )
            .expect("correlated cell load submits");

        let completion = loop {
            if let Some(completion) = coordinator.poll() {
                break completion;
            }
            thread::yield_now();
        };
        assert_eq!(completion.context(), context);
        assert_eq!(
            completion.into_result().expect("load succeeds").bytes,
            b"trace"
        );
    }

    #[test]
    fn cell_loads_reject_duplicates_and_report_cancellation() {
        let mut coordinator =
            CellLoadCoordinator::new(NonZeroUsize::new(1).expect("one worker is non-zero"));
        let token = CancellationToken::new();
        coordinator
            .submit(
                cell(2),
                1,
                request("cell-two", token.clone()),
                TestReader {
                    bytes: b"second".to_vec(),
                    worker_thread: Arc::new(Mutex::new(None)),
                },
                UncompressedDecoder,
            )
            .expect("cell load submits");
        assert_eq!(
            coordinator.submit(
                cell(2),
                2,
                request("cell-two-retry", CancellationToken::new()),
                TestReader {
                    bytes: b"retry!".to_vec(),
                    worker_thread: Arc::new(Mutex::new(None)),
                },
                UncompressedDecoder,
            ),
            Err(CellLoadSubmitError::DuplicateCell(cell(2)))
        );
        assert!(coordinator.cancel(cell(2)));
        assert!(!coordinator.cancel(cell(2)));
        assert!(token.is_cancelled());
        assert_eq!(coordinator.pending_len(), 0);

        let cancelled = CancellationToken::new();
        cancelled.cancel();
        coordinator
            .submit(
                cell(3),
                1,
                request("cell-three", cancelled),
                TestReader {
                    bytes: b"third".to_vec(),
                    worker_thread: Arc::new(Mutex::new(None)),
                },
                UncompressedDecoder,
            )
            .expect("pre-cancelled load submits");
        let completion = loop {
            if let Some(completion) = coordinator.poll() {
                break completion;
            }
            thread::yield_now();
        };
        assert_eq!(
            completion.into_result(),
            Err(CellLoadError::Asset(AssetLoadError::Cancelled))
        );
    }
}
