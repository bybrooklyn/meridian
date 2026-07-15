//! Bounded worker-pool primitives for engine background work.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use meridian_core::{OperationId, RuntimeEpoch, TraceId};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum TaskCompletion<T> {
    Completed(T),
    Panicked,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TaskClass {
    RealtimeAssist,
    FrameCritical,
    #[default]
    Streaming,
    Build,
    Background,
}

/// Correlation and invalidation metadata carried with worker jobs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskContext {
    pub class: TaskClass,
    pub operation_id: OperationId,
    pub trace_id: TraceId,
    pub runtime_epoch: RuntimeEpoch,
}

impl TaskContext {
    #[must_use]
    pub const fn new(
        class: TaskClass,
        operation_id: OperationId,
        trace_id: TraceId,
        runtime_epoch: RuntimeEpoch,
    ) -> Self {
        Self {
            class,
            operation_id,
            trace_id,
            runtime_epoch,
        }
    }
}

/// A typed handle for a submitted background task.
pub struct Task<T> {
    id: u64,
    context: Option<TaskContext>,
    receiver: Receiver<TaskCompletion<T>>,
}

impl<T> Task<T> {
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn context(&self) -> Option<TaskContext> {
        self.context
    }

    /// Waits until the task completes.
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::Panicked`] if the task closure panicked or
    /// [`TaskError::Disconnected`] if its completion channel closed early.
    pub fn wait(self) -> Result<T, TaskError> {
        match self.receiver.recv() {
            Ok(TaskCompletion::Completed(value)) => Ok(value),
            Ok(TaskCompletion::Panicked) => Err(TaskError::Panicked { task_id: self.id }),
            Err(_) => Err(TaskError::Disconnected { task_id: self.id }),
        }
    }

    /// Checks for a completed result without blocking.
    #[must_use]
    pub fn poll(&mut self) -> Option<Result<T, TaskError>> {
        match self.receiver.try_recv() {
            Ok(TaskCompletion::Completed(value)) => Some(Ok(value)),
            Ok(TaskCompletion::Panicked) => Some(Err(TaskError::Panicked { task_id: self.id })),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err(TaskError::Disconnected { task_id: self.id }))
            }
        }
    }
}

/// A fixed-size worker pool for non-render-thread engine jobs.
pub struct TaskPool {
    sender: Option<Sender<Job>>,
    workers: Vec<JoinHandle<()>>,
    next_task_id: AtomicU64,
    worker_count: NonZeroUsize,
}

impl TaskPool {
    /// Starts exactly `worker_count` worker threads.
    ///
    /// # Panics
    ///
    /// Panics if the operating system refuses to start a worker thread.
    #[must_use]
    pub fn new(worker_count: NonZeroUsize) -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(worker_count.get());

        for worker_index in 0..worker_count.get() {
            let receiver = Arc::clone(&receiver);
            let name = format!("meridian-worker-{worker_index}");
            let worker = thread::Builder::new()
                .name(name)
                .spawn(move || worker_loop(&receiver))
                .expect("Meridian worker thread must start");
            workers.push(worker);
        }

        Self {
            sender: Some(sender),
            workers,
            next_task_id: AtomicU64::new(0),
            worker_count,
        }
    }

    /// Starts a pool using the host's reported parallelism, reserving at least one worker.
    #[must_use]
    pub fn with_default_workers() -> Self {
        let worker_count = thread::available_parallelism().unwrap_or(NonZeroUsize::MIN);
        Self::new(worker_count)
    }

    #[must_use]
    pub const fn worker_count(&self) -> NonZeroUsize {
        self.worker_count
    }

    /// Submits a typed closure to the worker pool.
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::PoolClosed`] if the pool has already begun shutting down.
    pub fn submit<T, F>(&self, job: F) -> Result<Task<T>, TaskError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.submit_internal(None, job)
    }

    /// Submits a typed closure with cross-domain correlation metadata.
    ///
    /// # Errors
    ///
    /// Returns [`TaskError::PoolClosed`] if shutdown has begun.
    pub fn submit_correlated<T, F>(
        &self,
        context: TaskContext,
        job: F,
    ) -> Result<Task<T>, TaskError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.submit_internal(Some(context), job)
    }

    fn submit_internal<T, F>(
        &self,
        context: Option<TaskContext>,
        job: F,
    ) -> Result<Task<T>, TaskError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let task_id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let (result_sender, result_receiver) = mpsc::channel();
        let wrapped_job = Box::new(move || {
            let completion = match catch_unwind(AssertUnwindSafe(job)) {
                Ok(value) => TaskCompletion::Completed(value),
                Err(_) => TaskCompletion::Panicked,
            };
            let _ = result_sender.send(completion);
        });

        self.sender
            .as_ref()
            .ok_or(TaskError::PoolClosed)?
            .send(wrapped_job)
            .map_err(|_| TaskError::PoolClosed)?;

        Ok(Task {
            id: task_id,
            context,
            receiver: result_receiver,
        })
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.sender.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn worker_loop(receiver: &Arc<Mutex<Receiver<Job>>>) {
    loop {
        let job = receiver
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .recv();
        match job {
            Ok(job) => job(),
            Err(_) => break,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskError {
    PoolClosed,
    Panicked { task_id: u64 },
    Disconnected { task_id: u64 },
}

impl Display for TaskError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PoolClosed => write!(formatter, "Meridian task pool is closed"),
            Self::Panicked { task_id } => write!(formatter, "task {task_id} panicked"),
            Self::Disconnected { task_id } => {
                write!(formatter, "task {task_id} completion channel disconnected")
            }
        }
    }
}

impl Error for TaskError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> TaskPool {
        TaskPool::new(NonZeroUsize::new(2).expect("2 is non-zero"))
    }

    #[test]
    fn typed_tasks_complete_on_fixed_workers() {
        let pool = pool();
        assert_eq!(pool.worker_count().get(), 2);

        let first = pool.submit(|| 2_u32 + 3).expect("pool is open");
        let second = pool
            .submit(|| String::from("background"))
            .expect("pool is open");

        assert_eq!(first.wait().expect("task completed"), 5);
        assert_eq!(second.wait().expect("task completed"), "background");
    }

    #[test]
    fn correlated_submission_preserves_context_without_changing_simple_api() {
        let pool = pool();
        let context = TaskContext::new(
            TaskClass::Streaming,
            OperationId::new(2),
            TraceId::new(3),
            RuntimeEpoch::new(4),
        );
        let task = pool
            .submit_correlated(context, || 9_u32)
            .expect("pool is open");

        assert_eq!(task.context(), Some(context));
        assert_eq!(task.wait().expect("task completed"), 9);
        assert_eq!(pool.submit(|| 1_u32).expect("pool is open").context(), None);
    }

    #[test]
    fn poll_is_non_blocking_until_completion() {
        let pool = pool();
        let mut task = pool.submit(|| 42_u32).expect("pool is open");
        let mut result = None;
        for _ in 0..100 {
            if let Some(value) = task.poll() {
                result = Some(value);
                break;
            }
            thread::yield_now();
        }

        assert_eq!(result.expect("task completed").expect("task succeeded"), 42);
    }

    #[test]
    fn panicking_tasks_report_their_id() {
        let pool = pool();
        let task = pool
            .submit(|| -> u32 { panic!("test panic") })
            .expect("pool is open");
        let task_id = task.id();

        assert_eq!(
            task.wait().expect_err("panic must be reported"),
            TaskError::Panicked { task_id }
        );
    }

    #[test]
    fn dropping_pool_drains_submitted_work_before_shutdown() {
        let completed = Arc::new(AtomicU64::new(0));
        {
            let pool = pool();
            for _ in 0..8 {
                let completed = Arc::clone(&completed);
                pool.submit(move || {
                    completed.fetch_add(1, Ordering::Relaxed);
                })
                .expect("pool is open");
            }
        }

        assert_eq!(completed.load(Ordering::Relaxed), 8);
    }
}
