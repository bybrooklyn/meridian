# ADR-0027: Post-1.0 Competitive Performance and Quality Program

- Status: Adopted
- Date: 2026-07-16
- Spec version: v0.5
- Implementation status: Deferred
- Owners: release, performance, validation, product-quality, and subsystem leads
- Supersedes: none
- Superseded by: none

## Context

Meridian aims for materially better performance and quality, but a permanent or
global superiority promise cannot survive changing content, hardware, drivers,
engine versions, capability tiers, and artistic criteria. Internal benchmarks
alone cannot establish a fair cross-engine result.

## Decision

Adopt `PRG-REL-001` as a post-1.0 competitive performance and quality leadership
program governed by `COMPETITIVE_PERFORMANCE_AND_QUALITY_SPEC.md`.

The program:

- begins only after MS-10 and its independent entry gates;
- permits iso-quality performance, iso-cost quality, and matched-workflow claims
  only through immutable `CompetitiveBaselineRecord` evidence;
- preregisters corpus, parity, versions, hardware, settings, warmup, metrics,
  perceptual review, material threshold, expiry, and stop rules;
- retains raw samples, losing results, missing features, and lower-tier rows;
- requires scoped, expiring `CompetitiveClaim` records and retracts stale claims;
- cannot satisfy, block, or promote MS-00 through MS-10.

## Consequences

No current performance or quality superiority is claimed. The current active
package and 1.0 roadmap do not change. Contract design may be adopted before
MS-10, but competitive optimization and public comparative claims require a
future bounded planning review.

Marquee may later consume an approved evidence-bound claim. It cannot create,
broaden, renew, or approve one.

## Validation

`RG-REL-001` and `VAL-REL-001` require matched public/licensed corpora,
feature-parity matrices, raw distribution data, structural/reference/blinded
quality evidence, first-use and workflow measurements, accessibility/security/
provenance review, claim expiry, and negative-result retention.
