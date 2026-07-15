# ADR-0013: Typed Commands and Agent Access

- Status: Adopted
- Date: 2026-07-15
- Spec version: v0.3
- Implementation status: Command model planned/partial; agents deferred
- Owners: meridian-commands, editor, CLI, MCP, future agent API
- Supersedes: none
- Superseded by: none

## Context

Editor UI, CLI, Rust tools, MCP, and agents need one mutation model. If agents
receive private authority, they can bypass recovery, validation, approvals, and
auditing.

## Decision

One typed command/query registry serves UI, CLI, Rust tools, MCP, and agents.
Commands declare schema, capabilities, risk, preview, transaction policy,
checkpoint or undo behavior, thread domain, timeout, and audit fields. Agents
are optional clients with stricter capability checks and no private engine
backdoor.

Model prose never directly mutates project files.

## Current Evidence

- [Agent API, MCP, Ollama, and AI spec](../../../specs/AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md)
- [Editor and Meridian UI spec](../../../specs/EDITOR_AND_MERIDIAN_UI_SPEC.md)
- [Master specification](../../../specs/MERIDIAN_MASTER_SPEC.md)

## Intended v0.3 Links

- `specs/AGENT_API_MCP_OLLAMA_AND_AI_SPEC.md`
- `specs/EDITOR_AND_MERIDIAN_UI_SPEC.md`
- `specs/VERSION_CONTROL_COLLABORATION_AND_SYNC_SPEC.md`

## Consequences

Agent work is optional and zero-cost when disabled. External messages,
deployments, signing, account changes, destructive actions, and trust expansion
require the same or stricter authority as human UI/CLI workflows.

## Status Review

Review after typed command registry and VCS checkpoint foundations exist.
