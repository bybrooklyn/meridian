# Core Runtime, Tasks, and Platform Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Architecture](REPOSITORY_AND_CRATE_ARCHITECTURE.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md)

Version 0.2 · 2026-07-14 · Normative

## 1. Context and current state

The repository already contains a fixed-step core loop, diagnostics, task foundations, winit platform integration, ECS scheduling foundations, renderer extraction, and native smoke paths. These are Implemented foundations, not complete cross-platform or production runtime evidence.

Goals are deterministic ordering where promised, explicit clocks, bounded work, cancellation, generation-safe references, platform isolation, recoverable failures, and traceable frame ownership.

Non-goals: bit-identical floating-point behavior everywhere by default, one clock for all domains, unlimited catch-up, blocking async facades, or hidden platform fallbacks.

## 2. Runtime domains

| Domain | Clock | Mutation authority | Blocking allowed |
|---|---|---|---|
| Platform/event | monotonic wall | window/input state | short OS calls only |
| Fixed simulation | SimulationTick | ECS/world via commands | no |
| Presentation | PresentationTime | interpolation/camera/UI view | no |
| Render submission | RenderFrame | GPU resource and queue owner | no filesystem/network |
| Audio callback | SampleFrame | audio buffers only | never |
| IO/build workers | wall/deadline | artifacts and operation state | bounded yes |
| Network | NetworkTick | network snapshots/queues | no simulation mutation |

Cross-domain payloads carry source epoch and sequence. Consumers reject stale epochs and record drops.

## 3. Clocks

~~~rust
pub struct SimulationTick(pub u64);
pub struct RenderFrame(pub u64);
pub struct SampleFrame(pub u64);
pub struct NetworkTick(pub u64);
pub struct MonotonicNs(pub u64);

pub struct FrameTiming {
    pub now: MonotonicNs,
    pub simulation_steps: u8,
    pub interpolation_alpha: f32,
    pub discontinuity: Option<TimeDiscontinuity>,
}
~~~

The fixed-step accumulator clamps a configurable maximum wall delta and maximum catch-up count. Excess time produces a diagnostic and an explicit discontinuity; it is not silently simulated. Pause, step, replay, and time scale operate on simulation time, not audio device or wall clocks.

## 4. Main loop

1. poll platform events;
2. timestamp and normalize input;
3. apply lifecycle changes;
4. accumulate elapsed monotonic time;
5. execute zero to bounded fixed ticks;
6. commit command buffers at declared barriers;
7. publish immutable simulation snapshot;
8. build presentation state using interpolation;
9. extract render/audio/UI deltas;
10. submit domain work;
11. drain bounded diagnostics and deferred destruction;
12. sleep/yield according to selected pacing mode.

Reentrancy is forbidden. A lifecycle transition increments RuntimeEpoch and invalidates stale queued work.

## 5. Task model

~~~rust
pub enum TaskClass { RealtimeAssist, FrameCritical, Streaming, Build, Background }
pub struct TaskBudget { class: TaskClass, deadline: MonotonicNs, cpu_us: u32, bytes: u64 }
pub struct CancellationToken { operation: OperationId, generation: u32 }
pub struct TaskDescriptor {
    id: TaskId,
    parent: Option<TaskId>,
    budget: TaskBudget,
    affinity: Affinity,
    deterministic_key: Option<u64>,
}
~~~

Task submission requires a class, cost estimate, cancellation token, trace parent, and result channel. FrameCritical work is admitted against frame budget. Streaming is weighted by deadline and relevance. Build/background work yields before starving the runtime.

Deterministic parallel work partitions by stable key, records partition count, and merges in declared order. Work stealing MAY change execution order only when the output contract is order-independent.

## 6. Executor ownership

- one coordinator owns worker lifecycle;
- domain crates request tasks through meridian-tasks;
- no feature pack creates an unregistered recurring thread;
- render and audio real-time owners are separate from the general pool;
- blocking IO uses a bounded blocking lane;
- process workers are supervised with heartbeat, protocol version, resource limits, and restart policy.

Shutdown is structured: stop admission, cancel background, drain required commits, flush journals/traces, release domain owners, join workers, then destroy platform objects.

## 7. Memory

Memory categories are tagged: permanent, world, frame, render-frame, audio, streaming, build, script, network, and diagnostic. Each has current, peak, allocation count, and optional project/asset/cell attribution.

Rules:

- frame arenas reset only after all readers release the epoch;
- audio callback allocates nothing from general heaps;
- render upload staging is bounded and backpressured;
- large blobs are immutable and content-addressed;
- user-controlled sizes are checked before reserve/allocation;
- caches expose eviction reason and rebuild cost;
- out-of-budget behavior degrades/cancels with diagnostics before process exhaustion.

## 8. IDs and handles

Persistent StableId values are random or deterministic according to schema, never process addresses. Runtime Handle values are slot plus generation and cannot be serialized. WeakHandle upgrades can fail. Destruction queues retire a slot only after all owning epochs complete.

Stale handle behavior is a typed error in tools and a counted drop in hot paths where configured. Debug builds MAY capture creation/destruction operation IDs.

## 9. Platform contract

~~~rust
pub trait PlatformHost {
    fn capabilities(&self) -> PlatformCapabilities;
    fn poll(&mut self, sink: &mut dyn PlatformEventSink) -> Result<(), PlatformError>;
    fn create_window(&mut self, desc: WindowDescriptor) -> Result<WindowHandle, PlatformError>;
    fn monotonic_now(&self) -> MonotonicNs;
    fn launch_process(&self, desc: ProcessDescriptor) -> Result<ChildHandle, PlatformError>;
}
~~~

Portable contracts include windows/surfaces, DPI, input devices, clipboard, dialogs, paths, process supervision, timers, crash reporting, accessibility adapter hooks, audio device discovery, network capability, and power/thermal events.

Platform backends MUST report unsupported operations with capability detail and suggested fallback. OS paths and handles do not enter project data.

## 10. Input

Raw events are timestamped, device-scoped, and normalized without erasing source detail. Mapping produces semantic actions with phases, values, consumed state, and context stack. Text input and IME are distinct from key actions. Rebinding detects conflicts and is fully controller/keyboard accessible.

Input snapshots consumed by fixed simulation are immutable and tick-tagged. Replay captures normalized semantic inputs plus configuration hash. VR poses use predicted display timing and never masquerade as fixed-tick input.

## 11. Diagnostics

The runtime emits structured events:

~~~text
RuntimeEvent {
  code, severity, domain, operation_id, trace_id,
  frame_or_tick, message_key, fields, recovery_action,
  source, redaction_class
}
~~~

Required live panels/CLI output: frame pacing, fixed-step catch-up, task queues, cancellation, worker utilization, memory categories, stale handles, event latency, lifecycle state, backend capability, and last recovery.

## 12. Persistence and restart

Runtime state is reconstructed from project source, saved journal/snapshot, and compiled artifacts. Task queues and handles are not persisted. Long operations persist only explicit checkpoints whose input hashes and tool versions match on restart.

Crash context writes through a pre-opened bounded path where the platform permits. It contains no secrets or arbitrary asset bytes. A subsequent launch offers inspect, recover, safe mode, and open-copy choices.

## 13. Security and limits

Input, IPC, and file events are untrusted. Enforce message lengths, event rates, path roots, process arguments, environment allowlists, and timeout/cancellation. No shell string construction in build/process APIs. Diagnostic fields are classified public/project-sensitive/secret and redacted at sinks.

## 14. Quality tiers and fallbacks

- Interactive: normal editor/game scheduling.
- Deterministic capture: fixed pacing, deterministic partitions, recorded inputs.
- Headless: no window/render/audio requirement; simulation/build/server contracts remain.
- Low-power: reduced presentation rate and background budgets, unchanged simulation correctness.
- Safe mode: disables optional packs, custom scripts, third-party plugins, and cached artifacts.

Disabled domains register no recurring work.

## 15. Tests and benchmarks

- fixed-step sequences under jitter, stalls, pause, step, and clock discontinuity;
- bounded catch-up and spiral-of-death prevention;
- deterministic command merge across worker counts where promised;
- cancellation race and shutdown torture tests;
- stale-handle generation and epoch tests;
- allocator-free audio callback assertion;
- platform lifecycle and DPI/input fixtures;
- worker crash/restart and malformed IPC;
- minimal/headless no-window/no-GPU startup;
- frame/task/memory overhead calibrated on named tiers.

Acceptance evidence includes trace files and machine-readable summaries, not screenshots alone.

## 16. Phases

- Phase 1 completes platform/task/clock contracts on macOS and establishes Linux/Windows harnesses.
- Phase 2 adds render-domain pass timing and extraction/upload ownership evidence.
- Phases 3–8 add asset/world/input/audio/UI/save workloads without violating budgets.
- Phase 15 hardens deterministic modes and large-world scheduling.
- Phase 22 adds network clock/snapshot lanes.
- Phase 25 validates headless/minimal/optional builds.

## 17. Examples

End-to-end: a platform key event receives monotonic time, maps to MoveForward, enters tick 420 snapshot, mutates through a deterministic command buffer, publishes presentation snapshot 421, and appears in one trace across input/simulation/render.

Failure/recovery: a two-second debugger stall exceeds catch-up limits. The runtime records TimeDiscontinuity, runs only the bounded number of ticks, preserves input ordering, and presents a pacing diagnostic rather than attempting hundreds of steps.

Performance debug: a streaming burst consumes the FrameCritical queue. The trace attributes admitted microseconds and bytes by world cell and asset, shows missed deadline, and allows the editor to lower streaming budget or inspect the originating request reason.
