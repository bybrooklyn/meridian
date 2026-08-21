# Architecture records

Architecture decisions belong in the canonical
[decision directory](decisions/README.md). Keep engine/game dependency direction
explicit: the separate private game may depend on public engine APIs, while
engine crates must not depend on game crates.

version 1.0 uses ADRs for adopted architectural choices and the subsystem specs
for normative contracts. Architecture notes cannot promote implementation
maturity; evidence records under `governance/generated/` do that.
