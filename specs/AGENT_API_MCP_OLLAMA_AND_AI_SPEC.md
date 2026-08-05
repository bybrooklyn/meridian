# Agent API, MCP, Ollama, and AI Specification

[Master](MERIDIAN_MASTER_SPEC.md) · [Migration](SPEC_MIGRATION_AND_CONTRADICTIONS.md) · [Commands/UI](EDITOR_AND_MERIDIAN_UI_SPEC.md) · [Alluvium](PROCEDURAL_AUTHORING_SPEC.md) · [Marquee](MARQUEE_PROMOTIONAL_MEDIA_AND_EXPORT_SPEC.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md)

version 0.5 · 2026-07-15 · Normative architecture · Deferred to MS-08/MS-09

Documentation maturity: `ImplementationReady`. Implementation maturity:
`Planned`. Governing IDs: `REQ-AGT-001`, `WP-AGT-001`.

## 1. Principles

Agents are optional clients of Meridian’s typed command/query system. They receive no private engine backdoor, ambient project access, secret access, shell authority, or capability to waive tests/reviews. The engine, editor, game, build, VCS, and documentation work completely without AI.

Marquee defines a narrower post-1.0 AI profile: optional providers may draft text or return analysis/metadata suggestions, but may not generate or alter image, video, voice, music, or sound. Marquee suggestions remain untrusted until explicit human approval; cloud use discloses exactly what campaign data leaves the machine. This restriction is machine-validated and cannot be widened by an ordinary capability grant.

For Alluvium, agents may create or edit textual recipes, parameters,
constraints, tests, and candidate sets only through normal typed commands. An
opaque generated mesh, field, or binary cannot become source authority without
an editable recipe or an explicit reviewed promotion transaction.

MCP, Codex, Ollama, local models, and cloud providers are adapters. Local and cloud Ollama endpoints are distinct trust profiles. Web search is a separate network capability and is never inferred from model access.

## 2. Goals and non-goals

Goals: schema-derived tools/resources, least privilege, inspectable context, previews, transactions, checkpoints/rollback, audit, local-first inference, deterministic validations, and evaluation.

Non-goals: arbitrary shell/filesystem access, hidden prompt construction, automatic upload, treating model output as trusted, AI-only project formats, or calling a mode YOLO without bounded project capabilities and recovery.

## 3. Command/query registry

~~~text
CommandDescriptor {
  id: String,                          // namespaced, default max 128 bytes
  version: (u16 major, u16 minor, u16 patch),
  title: String,                       // default max 128 bytes, human-readable
  input_schema: SchemaRef,
  output_schema: SchemaRef,
  required_capabilities: Vec<CapabilityId>, // default max 16 per command
  risk: RiskClass,                     // Observe | Propose | Reversible | External |
                                        // DestructiveOrTrustChanging
  preview: PreviewPolicy,              // Required | Optional | NotApplicable
  transaction_policy: TransactionPolicy,
  undo_or_checkpoint: UndoPolicy,      // Inverse | Checkpoint | NotReversible
  thread_domain: ThreadDomain,
  timeout: Duration,                   // default reference bound 30 s, project-configurable
  audit_fields: Vec<AuditFieldSpec>,   // default max 32 recorded fields per invocation
}

QueryDescriptor {
  id: String,
  version: (u16 major, u16 minor, u16 patch),
  parameters: Vec<ParamDecl>,          // default max 16 parameters
  result_schema: SchemaRef,
  required_capabilities: Vec<CapabilityId>,
  cost_class: CostClass,               // Cheap | Moderate | Expensive
  sensitivity: SensitivityClass,       // Public | ProjectSensitive | Secret
  pagination: PaginationPolicy,        // default page size 100, default max 1,000 per page
  consistency: ConsistencyLevel,       // Snapshot | EventuallyConsistent
}
~~~

UI, CLI, Rust tools, MCP, and agents resolve these descriptors. An adapter may present fewer commands or stricter defaults but cannot change semantics.

## 4. Capability and approval model

Capabilities include project read by schema/object scope, propose edits, apply reversible edit, build/test/run, package, VCS operation, network provider, external link/search, mod/community, signing request, and destructive maintenance.

Grants specify principal/provider, project/workspace, command/query, object/path/schema scope, time/use limit, network destinations, data sensitivity, and approval rule. Agents cannot delegate or widen grants.

Risk classes:

- Observe: bounded read.
- Propose: creates preview only.
- Reversible: transaction with inverse/checkpoint.
- External: network/provider/remote side effect.
- Destructive or trust-changing: explicit user approval at execution.

## 5. Transaction flow

1. client negotiates registry/schema/API versions and identity;
2. requests query/context under capability;
3. proposes command with rationale and expected affected IDs;
4. engine validates schema, authority, current versions, budgets, and preconditions;
5. preview returns semantic diff, costs, permissions, build/test impact, and checkpoint plan;
6. approval policy resolves;
7. command executes through normal subsystem transaction;
8. validators/tests run as configured;
9. commit or rollback/checkpoint;
10. audit stores hashes, descriptor IDs, provider/model metadata, approvals, results, and redacted context classification.

Model prose never directly mutates project files.

## 6. MCP

The MCP server exposes:

- resources for specs, schemas, project graph, diagnostics, build/evidence, VCS views, docs, and selected source;
- tools generated from command/query registry;
- prompts/templates as non-authoritative conveniences;
- progress/cancellation and stable error codes.

Resource URIs are stable and capability checked. Pagination (default page size 100, default max 1,000 rows per page), byte limits (default max 1 MiB per resource read, default max 16 MiB per tool result before requiring pagination), snapshot/version tokens, and sensitivity labels are mandatory. Server does not expose raw secrets or arbitrary host files.

## 7. Context assembly

Context sources are explicit records:

~~~text
ContextItem {
  resource_id: ResourceUri,
  version_hash: u64,
  selected_ranges: Vec<ByteRange>,      // default max 32 ranges per item
  sensitivity: SensitivityClass,        // Public | ProjectSensitive | Secret
  provenance: ProvenanceRef,
  reason: String,                       // default max 256 bytes, shown in the disclosure UI
  token_or_byte_cost: u32,
}
~~~

A single agent turn's assembled context has a default reference ceiling of
128,000 tokens (or the active model's advertised context window, whichever
is smaller) before the assembler must drop or summarize lower-priority
`ContextItem`s rather than silently truncating mid-item.

Before a network provider call, the UI shows provider, endpoint class, retained/logging policy if known, exact sensitivity classes, and whether source/assets/traces are included. Users can remove items.

Prompt injection in project/docs/assets is untrusted content, never instruction authority. Tool policy comes from the registry/capability engine outside model context.

## 8. Ollama

Provider profiles separate:

- local Ollama endpoint: expected local transport, no implicit cloud routing;
- Ollama cloud: external provider and network trust;
- OpenAI-compatible endpoint behavior: protocol compatibility only, not identical capabilities/trust;
- web-search capability: independently authenticated and approved.

Runtime discovers model/capabilities rather than assuming context size, tools, embeddings, vision, structured output, or cloud access. Profiles record endpoint, model identity/digest where available, capability probe, data policy, timeouts, limits, and fallback.

## 9. Local index and retrieval

Ponder/agent retrieval uses versioned chunks from specs/docs/schemas/code symbols/diagnostics/examples under project policy. Embeddings and indexes are derived caches keyed by content, chunker, model, dimensions, and policy. They are rebuildable and never source authority.

Retrieval reports sources/scores/version and supports exact symbol/schema/diagnostic search without embeddings. Private content stays in project-local index unless explicitly exported.

## 10. Guarded autonomous project mode

A project MAY define a named automation profile sometimes called YOLO project mode. It is not unrestricted:

- isolated project/workspace/worktree;
- finite command allowlist and object/path scope;
- no signing keys, production deployment, credential changes, or arbitrary network;
- budgets for time/cost/build/model/changes;
- automatic checkpoints;
- required tests/validators;
- stop conditions and escalation;
- full audit and final semantic diff.

The profile cannot override host/user policy.

## 11. Editor and CLI

Editor shows agent session, provider/trust, granted capabilities, context, proposed plan, command previews, checkpoints, tests, costs, and audit. Human and agent edits are represented identically in VCS.

CLI/MCP support list descriptors, inspect schema, open session, grant/revoke scoped capability, query, preview, execute, cancel, rollback, and audit export. Secret values are passed by reference.

Approval, diff, context, cost, and audit surfaces must be keyboard navigable,
screen-reader meaningful, scalable, and free of color-only trust signals.
Beginner language may summarize a proposal, while expert inspection preserves
the exact typed commands and capability changes.

## 12. Threading and failure recovery

Provider calls and indexing run in optional worker processes/tasks. Results carry project/source/registry version; stale results cannot apply. Cancellation closes streams and leaves no partial transaction.

Provider crash/outage/rate limit produces a typed error and preserves the proposal/checkpoint. Local index corruption rebuilds. Malformed tool arguments fail before subsystem invocation.

## 13. Security

Tests cover prompt injection, tool-confusion, capability escalation, path traversal, symlink escape, secret exfiltration, oversized context/output, malicious MCP client/server, replay/stale approval, provider spoofing, model-generated unsafe source, and audit tampering.

Agents cannot approve their own trust expansion. External messages, PRs, deployments, account changes, and destructive operations require the same or stricter authority as human UI/CLI workflows.

## 14. Zero-cost behavior

When agent features are disabled: no provider dependencies/processes, MCP listener, embedding index, model downloads, network requests, agent panels, recurring tasks, or package chunks. Typed commands remain because humans/tools use them independently.

When Marquee AI is disabled, it additionally creates no campaign panel, analysis task, allocation, listener, network request, AI-assist record, or export content. Deterministic manual promotional workflows remain available.

## 15. Tests and evaluation

- registry parity across Rust/UI/CLI/MCP;
- schema/version/unknown command and stale resource tokens;
- capability/property/approval and rollback tests;
- provider local/cloud/web trust separation;
- offline Ollama and provider outage;
- prompt-injection/red-team corpus;
- context sensitivity/redaction/preview;
- Alluvium recipe schema, budget, deterministic replay, provenance/license,
  private-content redaction, candidate explanation, and no-opaque-source tests;
- task evaluation with expected commands/diffs/tests, not prose preference;
- latency, token/byte, index, memory, build/test cost attribution;
- checkpoint recovery after host/editor/provider crash.

Benchmarks report command-registry latency, context assembly and redaction cost,
index size/build time, provider round-trip distributions, tool execution time,
and editor responsiveness with the optional agent pack enabled and disabled.

## 15.1 Work package brief

Definition-of-Ready detail per [`IMPLEMENTATION_PLANNING_SPEC.md` §3](IMPLEMENTATION_PLANNING_SPEC.md).
No status change.

**`WP-AGT-001` — Typed command registry, MCP, and guarded agents**
Result: an agent can query a diagnostic, read selected spec/schema/code,
propose a bounded change, preview its semantic/source diff, get approval,
execute through normal commands, run validation, and checkpoint (§16's
end-to-end example) — with no privileged backdoor over UI/CLI/Rust tools
(§1). Owning contracts: `CommandDescriptor`, `QueryDescriptor` (§3),
`ContextItem` (§7). Entry conditions: stable command-registry, build-service,
and VCS checkpoint contracts (§16 — this package is `Deferred to MS-08/MS-09`
specifically because it depends on those, not on any Alluvium/Marquee
maturity). Deliverables: the shared registry UI/CLI/Rust/MCP/agents all
resolve identically (§3), the capability/approval model with risk classes
Observe/Propose/Reversible/External/Destructive (§4), the full transaction
flow in §5 (negotiate → query under capability → propose with rationale →
validate → preview diff/cost/checkpoint plan → approval → execute → validate
→ commit-or-rollback → audit), the MCP server surface (§6), context assembly
with mandatory sensitivity labeling before any network provider call (§7),
Ollama local/cloud provider-trust separation (§8), and the guarded autonomous
("YOLO") project mode with its full restriction list (§10). Non-goals: no
arbitrary shell/filesystem access, no automatic upload, no treating model
output as trusted, no AI-only project formats (§2); Marquee's narrower
post-1.0 AI profile (text/analysis only, no audiovisual generation) is
governed separately and this package does not widen it (§1, §14). Security:
agents cannot approve their own trust expansion; external messages, PRs,
deployments, and destructive operations require the same or stricter
authority as human workflows (§13). Tests: the full §15 list (registry
parity, schema/version/stale-token rejection, capability/approval/rollback,
provider trust separation, offline/outage handling, prompt-injection red-team
corpus, context redaction, task evaluation against expected commands/diffs,
checkpoint recovery after crash). Stop condition: any surface where agent
context cannot be proven classified/redacted before a network provider call
blocks that surface from shipping, not just from the default profile (§7,
§13's security tests). Next unblocked: MS-08/MS-09 profiles that assume a
working agent surface, and any later Marquee AI-assist work gated on this
package's capability/approval model existing first.

## 16. Delivery mapping and examples

MS-08/MS-09 implementation follows stable command-registry, build-service, and
VCS checkpoint contracts. Earlier development may use external developer
agents, but that is not an engine feature claim.

End-to-end: an agent queries a renderer diagnostic, reads selected spec/schema/code, proposes a bounded Alluvium material-recipe fix, previews semantic/source diff, provenance, license impact, cost, and tests, receives approval, executes normal commands, runs validation, and checkpoints the change.

Failure/recovery: a project file contains instructions to upload secrets. It is classified untrusted context; network/tool authority remains external, the request is denied/audited, and no secret enters the prompt.

Performance debug: indexing causes editor latency. Trace shows chunking/embedding workers exceeding Background budget; the user pauses the optional pack and verifies all recurring agent/index work disappears.
