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
  id, version, title, input_schema, output_schema,
  required_capabilities, risk, preview,
  transaction_policy, undo_or_checkpoint,
  thread_domain, timeout, audit_fields
}

QueryDescriptor {
  id, version, parameters, result_schema,
  required_capabilities, cost_class,
  sensitivity, pagination, consistency
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

Resource URIs are stable and capability checked. Pagination, byte limits, snapshot/version tokens, and sensitivity labels are mandatory. Server does not expose raw secrets or arbitrary host files.

## 7. Context assembly

Context sources are explicit records:

~~~text
ContextItem {
  resource_id, version_hash, selected_ranges,
  sensitivity, provenance, reason, token_or_byte_cost
}
~~~

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

## 16. Delivery mapping and examples

MS-08/MS-09 implementation follows stable command-registry, build-service, and
VCS checkpoint contracts. Earlier development may use external developer
agents, but that is not an engine feature claim.

End-to-end: an agent queries a renderer diagnostic, reads selected spec/schema/code, proposes a bounded Alluvium material-recipe fix, previews semantic/source diff, provenance, license impact, cost, and tests, receives approval, executes normal commands, runs validation, and checkpoints the change.

Failure/recovery: a project file contains instructions to upload secrets. It is classified untrusted context; network/tool authority remains external, the request is denied/audited, and no secret enters the prompt.

Performance debug: indexing causes editor latency. Trace shows chunking/embedding workers exceeding Background budget; the user pauses the optional pack and verifies all recurring agent/index work disappears.
