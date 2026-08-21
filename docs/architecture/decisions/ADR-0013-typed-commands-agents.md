# ADR-0013: Typed Commands and Agent Access

- Status: Adopted
- Date: 2026-07-15
- Refines: `MERIDIAN_SPECOMENT.md` sha256 `782d3110b89ac23f…`
- Retired v0.5 lineage: this ADR was adopted under v0.5 authority, which was retired at `PH-AUTH-004`. Section 0.5 ranks adopted ADRs directly below the specoment only where they cite the version they refine, so the citation above is what keeps this record in the authority order.
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

- [Agent API, MCP, Ollama, and AI spec](../../../MERIDIAN_SPECOMENT.md)

## Links
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`
- `MERIDIAN_SPECOMENT.md`

## Consequences

Agent work is optional and zero-cost when disabled. External messages,
deployments, signing, account changes, destructive actions, and trust expansion
require the same or stricter authority as human UI/CLI workflows.

## Status Review

Review after typed command registry and VCS checkpoint foundations exist.
