# Product Requirements Document

## Product statement

Quarantine Sandbox Runtime is a source-agnostic artifact-analysis execution plane. It accepts hostile bytes and optional bounded, non-secret submission context, produces deterministic evidence and runtime attestations, and returns analysis completeness to an authorized consumer.

Wardnet or another consumer remains the authority for maliciousness verdicts, incidents, quarantine policy, user notifications, blocking, and retention decisions.

## Users

- Security platform engineers integrating uploads, email attachments, connector artifacts, or repository artifacts.
- SOC analysts reviewing evidence and limitations.
- Runtime operators maintaining isolated Linux and Windows worker pools.
- Auditors verifying that analysis occurred under a declared policy and image revision.

## Foundation use cases

1. Submit bytes without requiring trigger-specific metadata, or attach a bounded source context when correlation metadata is available.
2. Carry only typed source-channel, original leaf file name, declared media type, opaque host artifact reference, and UTC submission-time metadata; never carry credentials, message bodies, raw URLs, or arbitrary attribute maps.
3. Compute immutable SHA-256 identity before any analyzer receives content.
4. Reject empty, oversized, path-like, sensitive-shaped, or malformed source metadata before analysis.
5. Classify common executable, archive, document, script, text, and unknown inputs from bytes; use the optional original file name only as an untrusted script-classification hint.
6. Invoke ordered static analyzer adapters.
7. Preserve analyzer failures as evidence rather than silently dropping them.
8. Return `completed` only when the requested foundation profile ran without analyzer failure.
9. Return `inconclusive` when dynamic analysis was requested but no dynamic worker is configured.
10. Prove that the foundation performed no execution, external network access, or credential use.
11. Require the consumer to make the final maliciousness verdict.

## Non-goals for the foundation PR

- No artifact detonation.
- No arbitrary command execution.
- No outbound lookup or reputation API.
- No YARA-X, capa, Ghidra, LIEF, or office macro adapter yet.
- No microVM, gVisor, Windows VM, eBPF, packet capture, or network sinkhole yet.
- No LLM classification.
- No automatic deletion, blocking, quarantine release, or incident closure.
- No database or object-storage persistence.
- No trigger-specific free-form metadata bag.

## Acceptance criteria

- Public contracts are versioned and match checked-in JSON Schemas.
- `bounded_source_context` is optional, fail-closed to unknown fields, and limited to 1,024 serialized UTF-8 bytes after per-field validation.
- Source timestamps follow the runtime's UTC-only RFC 3339 profile, and declared media types use bounded media-type syntax while remaining non-authoritative hints.
- Same request and bytes produce the same job and evidence identifiers.
- Unsupported dynamic profiles return evidence with `inconclusive`, never a false success.
- Analyzer failure is attributable to a producer and failure code.
- Serialized evidence contains no `malicious` or `verdict` field.
- Runtime boundary flags are all false in the foundation.
- Production Rust has complete statement, function, region, and branch coverage targets.
- Public Rust API documentation is complete.
- All action dependencies are pinned by full commit SHA.
