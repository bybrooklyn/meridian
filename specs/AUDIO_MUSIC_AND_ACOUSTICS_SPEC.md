# Wavefront Audio, Music, Voice, and Acoustics Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [ADR-0020](../docs/architecture/decisions/ADR-0020-wavefront-and-collective.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [Runtime/tasks](CORE_RUNTIME_TASKS_AND_PLATFORM_SPEC.md) · [Assets/world/save/package formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md) · [Isobar](ISOBAR_WEATHER_AND_ATMOSPHERE_SPEC.md) · [Validation](TESTING_BENCHMARKS_AND_VALIDATION.md) · [Delivery roadmap](DELIVERY_ROADMAP.md)

Status: version 0.5 normative architecture for planned audio work, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Scaffold`.
Governing IDs: `REQ-AUD-001`, `WP-AUD-001`.

Architecture status: Wavefront is `Adopted` by ADR-0020. Current implementation status: `engine/meridian_audio` is a scaffold only. The subsystem name does not rename the crate in this pass. This document specifies planned architecture and transitional gates. It does not claim an implemented mixer, decoder, device backend, music system, voice stack, or acoustic solver.

## 1. Context

Wavefront is Meridian's audio subsystem. It must support the opening forest first: reliable device output, streamed ambience, footsteps, authored environmental one-shots, simple spatialization, forest/field acoustic contrast, title/static transitions, and diagnostics that reveal underruns. Advanced hybrid acoustics, adaptive music authoring, platform object audio, plugin hosting, real-time geometric propagation, and voice communication are independently optional packs.

Wavefront owns capture devices, permission-aware microphone acquisition, encoding/decoding seams, jitter-to-audio buffering, DSP, mute/gain, spatial playback, accessibility hooks, mixing, and output devices. [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) owns optional voice-room membership, identity, permissions, block/mute policy synchronization, provider coordination, and moderation workflow. [Networking](MULTIPLAYER_AND_SERVER_SPEC.md) owns transport. No subsystem may collapse those three authorities into a provider SDK surface.

The version 0.1 documents named CPAL and Symphonia as plausible low-level foundations and required forest audio, weather mixing, silence transitions, spatialization, and dynamic music. Version 0.3 refines that into a real-time-safe audio callback, immutable compiled DSP graphs, integer sample clocks, chunked streaming, explicit device backends, and scalable acoustic feature tiers.

## 2. Goals and Non-Goals

Goals:

- Make the audio callback predictable: no blocking I/O, unbounded allocation, ordinary mutex waits, direct logging, or worker waits.
- Provide a simple mixer workflow for beginners while preserving graph-level control for audio programmers.
- Use integer sample time for music transitions, automation, fades, and gameplay audio commands.
- Stream long audio assets through bounded read/decode/ring-buffer stages.
- Build acoustic scenes from material and world facets, not from render triangles by default.
- Degrade cleanly under load without blocking the callback.
- Keep expensive acoustics and generative music zero-cost when disabled.
- Keep microphone capture and voice processing absent until explicitly enabled and permissioned.

Non-goals for the opening slice:

- No full geometric acoustic solver.
- No VST3, Audio Unit, CLAP, LV2, or DAW plugin hosting.
- No generative music runtime.
- No mandatory HRTF dependency.
- No fixed codec commitment before licensing/platform research.
- No claim that Meridian acoustics are physically validated until measured against fixtures.
- No always-on microphone, ambient voice account, mandatory voice provider, or Meridian-hosted voice service.

## 3. Ownership and Crate Boundaries

Authoritative source data:

- Source audio documents, cues, music graphs, mixer graphs, room/acoustic annotations, and material acoustic facets live in the project source world and asset documents defined by [Assets/world/save/package formats](ASSET_WORLD_SAVE_AND_PACKAGE_FORMATS.md).
- Built audio packets, seek tables, compiled DSP graphs, impulse summaries, and acoustic acceleration data are caches or package artifacts.
- Runtime graph snapshots and device state are process-local, not authoritative source.

Planned ownership:

| Area | Owning crate or tool | Status | Notes |
|---|---|---|---|
| Public audio IDs, units, bus paths, command schemas | `meridian-audio` | planned | Must not expose CPAL, Symphonia, or backend handles. |
| Device abstraction and callback host | `meridian-audio` | planned | Backend-specific code is private modules. |
| DSP graph compiler/runtime | `meridian-audio` | planned | Immutable compiled snapshots cross into callback. |
| Streaming decode workers | `meridian-audio` + `meridian-assets` | planned | Asset system owns packaged bytes; audio owns decode policy. |
| Acoustic scene and propagation | future `meridian-acoustics` or `meridian-audio::acoustics` | planned | May split once hybrid acoustics becomes large. |
| Music transport graph | `meridian-audio` | planned | Uses sample clock plus music beat/bar time. |
| Voice capture, local DSP, decode, jitter-to-mixer bridge, and playback | `meridian-audio` | planned | Wavefront owns audio behavior; transport and room policy remain external. |
| Voice rooms, identity, permissions, provider session, reporting | Collective | planned | Collective never owns callback, device, or mixer state. |
| Audio editor panels/tools | `editor/meridian_editor` and future audio tools | planned | Editor may depend on engine audio schemas. |
| Project Meridian cues | external private game repository | planned | Game content never owns engine mixer policy. |
| The Alluvium Engine | Generated acoustic material/region/portal/obstruction source facets and provenance | planned | Audio retains live propagation, mixer, voice, and device authority. |

Allowed dependencies:

- `meridian-audio` may depend on `meridian-core`, `meridian-diagnostics`, `meridian-assets`, `meridian-tasks`, and feature-gated backend/decode libraries.
- `meridian-audio` may read immutable weather, world, material, and transform snapshots through public contracts.
- `meridian-audio` may consume versioned Alluvium acoustic artifacts; it must
  not invoke authoring/compiler internals from the callback or runtime graph.
- `meridian-renderer`, `meridian-isobar`, and `meridian-physics` must not depend on `meridian-audio`; cross-system coupling travels through events, fields, and material facets.

Invalid dependencies:

- Invalid: `meridian_renderer -> meridian_audio` to decide visual timing from bus loudness.
- Invalid: a consumer-game crate depending on `cpal` to open an output device directly.
- Invalid: `meridian-audio` public APIs exposing `cpal::Device`, decoder objects, platform handles, or editor widget types.
- Invalid: Collective or a provider SDK writing directly to the Wavefront callback or opening a microphone without a Wavefront permission transaction.

Dependency direction:

```text
external consumer-game content
  -> meridian-audio public cue/bus/music APIs
  -> meridian-assets public asset IDs

meridian-audio
  -> meridian-assets packaged bytes
  -> meridian-tasks workers
  -> meridian-diagnostics counters
  -> meridian-core IDs, math, units

editor tools
  -> meridian-audio schemas and offline compilers
  -> no callback-owned mutable state
```

## 4. Public Types and Data Structures

The following are Rust-like schemas, not current implementation. Names are
stable intent; exact module paths can change through approved work packages.

```text
struct AudioBusId(u64 stable_hash);
struct AudioCueId(u128 persistent_uuid);
struct AudioStreamId { slot: u32, generation: u32 }
struct AudioGraphId(u128 persistent_uuid);
struct AcousticZoneId(u128 persistent_uuid);
struct VoiceParticipantId(u128 persistent_uuid);

struct AudioTimestamp {
    sample: u64,
    rate: u32,
}

struct AudioClockMap {
    fixed_tick: u64,
    fixed_tick_rate_num: u32,
    fixed_tick_rate_den: u32,
    audio_sample_at_tick: u64,
    drift_ppm_estimate: f32,
}

struct CompiledAudioGraph {
    sample_rate: u32,
    block_size: u32,
    nodes: Box<[CompiledNode]>,
    schedule: Box<[NodeIndex]>,
    buffers: AudioBufferPlan,
    parameters: ParameterTable,
    latency_samples: u32,
    worst_case_work_units: u32,
}

struct AudioCommandBatch {
    target_sample_start: u64,
    commands: Box<[AudioCommand]>,
    overflow_policy: QueueOverflowPolicy,
}

enum AudioCommand {
    PlayCue { cue: AudioCueId, emitter: EmitterRef, bus: AudioBusId, gain_db: f32 },
    StopCue { instance: AudioStreamId, fade_samples: u32 },
    SetParameter { target: ParameterRef, value: f32, ramp_samples: u32 },
    SwapGraph { graph: Arc<CompiledAudioGraph>, effective_sample: u64 },
}

struct VoiceAudioPolicy {
    capture_permission: PermissionState,
    push_to_talk: bool,
    local_mute: bool,
    input_gain_db: f32,
    processing_profile: VoiceProcessingProfile,
    accessibility_cues: AccessibilityCuePolicy,
}

struct AcousticPatch {
    bounds: Aabb,
    material: AcousticMaterialId,
    room: Option<AcousticZoneId>,
    portal: Option<PortalId>,
    geometry_lod: u8,
}
```

Hot fields use SoA arrays where the callback iterates every block: per-voice gains, source buffer cursors, bus accumulators, spatial parameters, and active node state. Cold fields such as debug names, source paths, provenance, editor notes, and waveform thumbnails stay outside callback memory and are stripped or interned in shipping builds.

Handle rules:

- Runtime handles include generation counters and become invalid after graph/device reset.
- Persistent IDs are serialized in source documents and saves.
- Callback memory is reclaimed only after an epoch proves no callback can still hold the previous snapshot.
- Asset-derived buffers are reference-counted immutable pages; decoded PCM rings are owned by stream instances.

## 5. Runtime Pipeline

Per engine frame:

1. Game, world, weather, animation, and interaction systems emit audio intent into command buffers.
2. The audio bridge converts fixed ticks and presentation estimates into `AudioTimestamp` values through `AudioClockMap`.
3. The mixer command queue validates cue IDs, bus paths, parameters, and deadlines.
4. Streaming jobs are prioritized by start sample, audibility, and preload policy.
5. Prepared immutable graph/resource snapshots are published for block-boundary adoption.
6. Diagnostics collect non-callback counters and enqueue callback-safe metric snapshots.
7. If voice is enabled, a permissioned capture worker and NET/Collective bridge publish bounded decoded frames and participant policy into ordinary Wavefront stream inputs.

Per audio callback block:

1. Read the current immutable `CompiledAudioGraph`.
2. Drain bounded commands with target samples inside or near the block.
3. Apply graph swaps only at safe sample boundaries.
4. Pull decoded PCM from stream rings; substitute silence or low-cost loop fallback on starvation.
5. Process scheduled DSP nodes in topological order.
6. Apply spatialization/acoustic sends according to active tier.
7. Mix buses, limit according to configured safety policy, and write device samples.
8. Publish callback duration, underrun, and overflow counters through a non-blocking channel.

Graph compilation:

```text
resolve node definitions and versions
-> validate ports and channel layouts
-> reject illegal cycles unless a delay node makes feedback explicit
-> choose sample-rate conversion and block adaptation
-> topologically schedule nodes
-> plan intermediate buffers and bus accumulators
-> compute latency compensation
-> precompute parameter routes
-> estimate worst-case work
-> publish immutable graph with monotonically increasing epoch
```

Music transition scheduling:

```text
read transport state
-> evaluate transition rules before latest safe point
-> choose beat/bar/phrase boundary
-> subtract decoder preroll and graph latency
-> enqueue stream starts and fades at exact sample timestamps
-> verify all required stream rings will be ready
-> fall back to musically legal hold/loop when not ready
```

## 6. Threading, Memory, and Lifetime

Latency-critical:

- Device callback, active mixing, sample-rate conversion, and minimal spatialization prefer performance cores where the OS allows hints.
- Callback code uses bounded stack/local scratch, preallocated arenas, and immutable snapshots.

Parallel/background:

- Decode, waveform analysis, loudness scans, graph compilation, acoustic baking, impulse reduction, and preview rendering can run on worker threads and efficiency cores.
- Long acoustic bakes and imports run in isolated worker processes when crashes could corrupt editor state.

Synchronization:

- Callback consumes single-producer/single-consumer or multi-producer bounded queues with explicit overflow counters.
- Ordinary mutexes are allowed in editor/offline compilers, not in the callback path.
- Graph snapshots use atomic pointer swap plus epoch reclamation.
- Stream rings use bounded lock-free or wait-free structures only where contention is proven; otherwise use worker-owned rings handed off by atomic indices.

Cancellation and shutdown:

- Decode jobs poll cancellation between chunks.
- Device loss publishes a `DeviceUnavailable` diagnostic, drops to silent virtual device mode, and keeps graph state alive for recovery.
- Editor crashes in import/acoustic workers preserve source documents and discard partial cache writes unless transaction commit markers are complete.

Deterministic modes:

- Sample scheduling is deterministic from source documents, save state, and command order.
- Callback thread timing is not deterministic; tests compare emitted sample events and mixed output fixtures under a virtual device.

## 7. Persistence, Versioning, and Compatibility

Source documents:

- Mixer graphs, cue sheets, music graphs, acoustic zones, and material acoustic facets are versioned source documents.
- Unknown fields are preserved by editor migrations where the document framework supports it.
- Persistent cue/bus/zone IDs never derive solely from file paths.

Built artifacts:

- Audio assets compile into package facets with codec, sample rate, channel layout, seek table, loudness metadata, loop points, markers, and checksums.
- Compiled graphs include source graph ID, compiler version, target sample rate, feature flags, and dependency hashes.
- Acoustic caches include world region ID, material-facet versions, geometry LOD, weather coupling assumptions, and bake algorithm version.

Compatibility:

- A newer editor can open older graph documents through explicit migrations.
- A runtime rejects unsupported graph/artifact versions with actionable diagnostics and fallback silence, not undefined output.
- Saves record active cue instances, music transport state, listener state, and enough clock mapping to recover cleanly after resume; they do not serialize backend device handles.

## 8. Editor, CLI, MCP, and Workflows

Beginner workflow:

1. Import audio files.
2. Drag ambience/footstep/cue assets into a scene or zone.
3. Choose simple spatial behavior: 2D, 3D point, area ambience, room reverb.
4. Press Play and hear through the same runtime mixer.
5. Use warnings such as "stream preload too late" or "cue has no fallback" with one-click fixes.

Expert workflow:

1. Open mixer graph, DSP graph, music transport, waveform, spectrogram, meters, and acoustic debug views.
2. Pin sample positions, bus loudness, clock drift, graph node cost, propagation path counts, and stream rings.
3. Author exact transitions, sidechains, parameter automation, HRTF/ambisonic settings, and reverb zones.
4. Run offline loudness, loop, underrun, and acoustic bake validation.

CLI commands, planned:

```text
meridian audio inspect <project> --cue <id>
meridian audio build <project> --platform <target>
meridian audio validate <project> --opening-forest
meridian audio render-fixture <project> --fixture <name>
meridian audio recover-cache <project>
```

MCP/agent surface:

- Agents use the same command/query registry as the editor.
- Allowed operations include listing cues, reading diagnostics, proposing graph edits, running validation, and creating checkpoints.
- Agents cannot access microphone input, system devices, or external plugin scans without explicit permission.
- Voice tools expose permission, mute, capture-device, jitter, packet-loss concealment, and accessibility state without revealing private speech content.

## 9. Diagnostics, Failure Recovery, and Security

Diagnostics:

- callback duration distribution;
- underruns and late commands;
- graph node cost;
- stream starvation and decode queue depth;
- voice and bus counts;
- clock drift;
- per-bus loudness;
- acoustic ray/path counts;
- convolution cost;
- device changes and backend fallback.
- voice capture permission, jitter, concealment, mute, and participant-stream health without recording speech by default.

Failure recovery:

- Stream starvation crossfades to silence or an authored fallback loop.
- Graph compile failure keeps the previous valid graph and reports the rejected nodes.
- Device loss enters silent virtual device mode and attempts controlled backend recovery.
- Corrupt audio package chunks mark affected cues unavailable while keeping the project open.
- Callback queue overflow drops or coalesces commands according to the command type and records the loss.

Security:

- Audio importers and plugin hosts are not trusted.
- Optional plugin hosting must run isolated where practical and must never execute during the opening-slice baseline.
- Source audio metadata is treated as untrusted input.
- MCP and agent commands require project capability checks and audit entries.
- Voice capture is opt-in, visibly indicated, revocable, and never activated by joining a Collective room alone.

## 10. Capability Tiers and Zero-Cost-Disabled Behavior

Baseline opening tier:

- Device output.
- Bus mixer.
- Streamed ambiences and one-shots.
- Footstep variation.
- Simple point/area spatialization.
- Occlusion by simple ray or zone flag where available.
- Reverb zones or lightweight parameterized reverb.
- Diagnostics overlay.

Optional tiers:

- HRTF convolution.
- Ambisonics and object output.
- Adaptive music transport graph.
- Hybrid baked/real-time acoustics.
- Dynamic diffraction and early reflections.
- Convolution reverb from impulse summaries.
- Plugin hosting.
- Generative music.

Zero-cost-disabled tests:

- Disabled acoustics allocate no acoustic scene, schedule no propagation work, and add no scene traversal.
- Disabled music graph schedules no transport evaluation.
- Disabled HRTF creates no convolution kernels.
- Disabled voice creates no capture stream, encoder/decoder, jitter buffer, transport binding, provider SDK, microphone permission request, or package chunk.
- Headless/server builds exclude client audio device backends and source media not needed for validation.

## 11. Algorithm Alternatives and Research Gates

Spatialization:

- Equal-power pan: cheap, stable fallback, limited spatial realism.
- Speaker panning: good for known layouts, requires channel mapping.
- HRTF: better headphone localization, costs convolution and dataset management.
- Platform object audio: high integration value on supported platforms, portability risk.

Acoustics:

- Zone/portal reverb: controllable, cheap, limited geometric fidelity.
- Precomputed probes/impulse summaries: good static-world quality, bake cost and storage.
- Real-time geometric tracing: supports dynamic paths, CPU/GPU budget and stability risk.
- Wave-based methods: stronger diffraction/low-frequency plausibility, usually too expensive for broad real-time use.

Music:

- Authored stems with rule transitions: opening baseline candidate, predictable and inspectable.
- Procedural/generative music: later research, must be opt-in and auditable.

Research gates:

- MS-04/MS-06/MS-08 selects initial device/decode/mixer implementation details through virtual-device fixtures and opening-forest cue tests.
- MS-08 compares zone/portal, probe, and selected real-time geometric acoustic prototypes on shared forest/building fixtures.
- No production claim of physical acoustic accuracy is allowed without measured fixtures and documented error bounds.

## 12. Tests, Benchmarks, and Acceptance Evidence

Tests:

- Graph compile validation: cycles, ports, channel layouts, latency compensation.
- Virtual callback: no allocations or blocking in instrumented builds.
- Clock mapping: fixed tick to sample timestamp under drift.
- Streaming: seek tables, late preloads, corrupted pages, cancellation.
- Save/resume: music transport and active cue recovery.
- Disabled-feature tests for zero recurring work.

Benchmarks:

- Opening forest ambience with footsteps, wind, canopy, branch one-shots, static/title transition.
- Stream starvation stress with slow storage simulation.
- Graph node cost fixture with representative mixer and spatialization loads.
- Acoustic prototype fixture for forest edge and room/portal transition.

Acceptance evidence:

- Demo capture showing no callback underruns in the opening slice on target hardware.
- Diagnostic JSON with callback distribution, voice counts, stream queue depth, and fallback counts.
- Recovery demo for device loss or missing stream chunk.
- Documentation and editor tooltips for beginner cue placement and expert mixer diagnostics.

## 13. Delivery Mapping

- MS-06/MS-07: minimum Wavefront audio: output, mixer, streaming, simple spatialization, footstep/event cues, forest/field bus transitions, diagnostics. Voice is not a Project Meridian prerequisite.
- MS-04/MS-06/MS-08: robust mixer, DSP graph compiler, adaptive music foundation, virtual-device tests, source/built audio document versions.
- MS-08: hybrid acoustics, acoustic authoring, bake/probe tooling, advanced profiling.
- MS-08/MS-09: MCP/agent audio commands with audit and permission checks; optional Wavefront voice-device/runtime contracts may support Collective when independently selected.
- Later optional packs: plugin hosting, generative music, platform object audio depth, advanced dynamic propagation.

## 14. Examples

End-to-end opening example:

```text
Designer imports forest_wind.flac, wet_leaf_step_*.wav, branch_shift.wav.
-> Editor creates source cue documents with stable CueIds and bus paths.
-> Build produces platform audio facets with seek tables and loudness metadata.
-> Opening Zone A loads ambience rings before fade-in.
-> Runtime schedules forest wind at sample S, footsteps by material at sample S+n.
-> Zone E schedules static buildup and title cut at an exact sample boundary.
-> Save records active ambience and music/static transport state.
```

Failure/recovery example:

```text
Decoded PCM ring for distant canopy underruns.
-> Callback records stream_starvation and substitutes authored low-cost loop for this block.
-> Worker raises priority for the missing stream pages.
-> Editor/runtime diagnostic names the cue, package chunk, deadline, and storage latency.
-> If recovery succeeds, crossfade back to the intended stream.
```

Performance-debug example:

```text
Profiler shows callback p99 near the device deadline.
-> Audio diagnostics sort node cost and voice groups.
-> Expert view reveals HRTF convolution enabled for distant insects.
-> User switches distant ambience to stereo area bed.
-> Validation confirms no HRTF kernels or propagation work remain for that bus on the low tier.
```
Audio authoring and runtime settings expose captions and non-speech cues,
visual alternatives for gameplay-critical sound, mono/downmix and dynamic-range
controls, hearing-device-safe defaults, keyboard/controller operation, and
screen-reader labels for graph and mixer state. Beginner cue placement and
expert sample/graph diagnostics operate on the same source model.
