# Integrity, Anti-Cheat, and Moderation Specification

[Master index](MERIDIAN_MASTER_SPEC.md) · [Security](SECURITY_SIGNING_UPDATES_AND_SUPPLY_CHAIN.md) · [Networking](MULTIPLAYER_AND_SERVER_SPEC.md) · [Collective](COLLECTIVE_ONLINE_SERVICES_SPEC.md) · [Modding](MODDING_AND_COMMUNITY_LIBRARY_SPEC.md) · [Post-1.0 programs](DELIVERY_ROADMAP.md)

Status: version 0.5 post-1.0 research authority, 2026-07-15.

Documentation maturity: `ResearchReady`. Implementation maturity: `Deferred`.
Governing IDs: `REQ-INT-001` through `REQ-INT-003`; `PRG-INT-001`.

Current implementation status: no Meridian anti-cheat client, integrity attestation service, behavioral detection system, sanction service, kernel component, or operated moderation platform exists. Baseline server authority and security hardening remain in NET/SEC/COL.

## 1. Authority and Principles

The `INT` domain owns future game-integrity policy contracts, client/server integrity signals, attestation abstractions, detection evidence, review/appeal records, false-positive controls, and compatibility with mods and accessibility software. Collective owns moderation cases and sanctions as an online-service workflow. SEC owns general trust, signing, sandboxing, update, and supply-chain policy. Games own competitive rules and acceptable modifications.

Principles:

- server authority and protocol validation precede invasive client techniques;
- every signal has provenance, confidence, scope, retention, and an appeal path;
- no automatic permanent sanction from one opaque or unreviewed signal;
- privacy, accessibility, modding, platform policy, and false-positive cost are requirements;
- kernel-level or similarly invasive components are neither assumed nor preferred and require a separate exceptional ADR and evidence;
- security claims never disclose exploit-enabling operational detail in public evidence.

## 2. Planned Contracts

```text
IntegrityPolicy { project, mode, allowed_mods, required_signals, actions, appeal }
IntegritySignal { id, subject, source, observation, confidence, provenance, retention }
AttestationResult { platform, capability, claims, freshness, trust, limitations }
DetectionAssessment { signals, model_or_rules, confidence, alternatives, reviewer_state }
IntegrityActionRequest { scope, duration, reason_code, evidence_refs, authority }
IntegrityAuditRecord { decision, reviewers, policy_revision, notifications, appeal }
```

Raw provider handles, secrets, personal data, detector internals, and exploit samples do not cross public gameplay APIs. Accessibility tools and approved mods are explicit policy inputs, not assumed cheats.

## 3. Ordered Decision Pipeline

```text
collect minimum authorized server/client signals
-> validate schema, provenance, freshness, and tamper state
-> correlate under project policy
-> classify confidence and alternative explanations
-> apply reversible protective action when necessary
-> require authorized review for serious sanctions
-> notify with safe reason and appeal route
-> retain/delete evidence by policy
-> measure false positives, reversals, and disparate impact
```

Detection failure, service outage, unsupported platform, missing attestation, privacy refusal, stale signal, model drift, and appeal are typed states. A missing optional signal cannot silently become proof of cheating.

## 4. Security, Privacy, and Failure

Threats include malicious clients, compromised servers, replay/tampering, detector evasion, provider compromise, insider abuse, evidence forgery, harassment through reports, privacy leakage, accessibility-tool conflicts, and supply-chain attacks. Mitigations include server authority, signed artifacts, least privilege, rate limits, isolated analysis, audit separation, red-team fixtures, key rotation, bounded retention, access review, and incident response.

Diagnostics and public reports aggregate false-positive, review, appeal, latency, availability, and coverage metrics without exposing detector bypass details or personal evidence. Private incident data uses explicit access, encryption, provenance, and deletion.

## 5. Requirements and Program Gates

- `REQ-INT-001`: typed provenance-bearing integrity signals and evidence-based decisions with measured false-positive and evasion testing.
- `REQ-INT-002`: privacy, accessibility, modding, platform, review, notification, retention, and appeal safeguards.
- `REQ-INT-003`: server-authority-first architecture, provider-neutral boundaries, secure diagnostics, and zero-cost-disabled behavior.
- `PRG-INT-001`: post-1.0 advanced integrity program; it cannot satisfy or block MS-00 through MS-10.

Entry requires mature NET authority, SEC signing/update and incident response, Collective moderation workflows, mod capability policy, privacy/legal review, and funded maintenance. Tests include adversarial clients, replay/tamper, provider outage, false positives, accessibility/mod compatibility, appeal reversals, secrets/redaction, update rollback, and bypass-resistant black-box evaluations. No vendor marketing claim counts as Meridian evidence.

## 6. Accessibility and Zero-Cost Behavior

Reports, notices, account restrictions, evidence summaries, and appeals are accessible and localized. Integrity policy explicitly accommodates remapping, assistive input, overlays, screen readers, speech tools, and approved accessibility modifications.

Offline/noncompetitive projects activate no integrity collection, attestation, background service, network request, detector, or package artifact. Collective moderation modules and general SEC hardening can exist independently.

## 7. Examples

End to end: an authoritative server detects impossible command timing, combines it with protocol evidence, applies a temporary protective restriction, and sends the case to reviewed moderation with an appeal path.

Failure: platform attestation is unavailable. Policy records `UnsupportedCapability`; the project applies its declared fallback rather than labeling the player malicious.

Performance debug: an integrity latency trace separates collection, transport, correlation, review queue, and sanction propagation while redacting sensitive detector details.
