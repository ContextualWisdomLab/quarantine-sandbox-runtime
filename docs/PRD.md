# Product Requirements Document

## Product statement

Quarantine Sandbox Runtime is a source-agnostic artifact-analysis execution plane. It accepts hostile bytes and bounded submission context, produces deterministic evidence and runtime attestations, and returns analysis completeness to an authorized consumer.

Wardnet or another consumer remains the authority for maliciousness verdicts, incidents, quarantine policy, user notifications, blocking, and retention decisions.

## Users

- Security platform engineers integrating uploads, email attachments, connector artifacts, or repository artifacts.
- SOC analysts reviewing evidence and limitations.
- Runtime operators maintaining isolated Linux and Windows worker pools.
- Auditors verifying that analysis occurred under a declared policy and image revision.

## Foundation use cases

1. Submit bytes plus source context without coupling to a trigger product.
2. Compute immutable SHA-256 identity before any analyzer receives content.
3. Reject empty, oversized, or malformed metadata.
4. Classify common executable, archive, document, script, text, and unknown inputs.
5. Invoke ordered static analyzer adapters.
6. Preserve analyzer failures as evidence rather than silently dropping them.
7. Return `completed` only when the requested foundation profile ran without analyzer failure.
8. Return `inconclusive` when dynamic analysis was requested but no dynamic worker is configured.
9. Prove that the foundation performed no execution, external network access, or credential use.
10. Require the consumer to make the final maliciousness verdict.

## Non-goals for the foundation PR

- No artifact detonation.
- No arbitrary command execution.
- No outbound lookup or reputation API.
- No YARA-X, capa, Ghidra, LIEF, or office macro adapter yet.
- No microVM, gVisor, Windows VM, eBPF, packet capture, or network sinkhole yet.
- No LLM classification.
- No automatic deletion, blocking, quarantine release, or incident closure.
- No database or object-storage persistence.

## Acceptance criteria

- Public contracts are versioned and match checked-in JSON Schemas.
- Same request and bytes produce the same job and evidence identifiers.
- Unsupported dynamic profiles return evidence with `inconclusive`, never a false success.
- Analyzer failure is attributable to a producer and failure code.
- Serialized evidence contains no `malicious` or `verdict` field.
- Runtime boundary flags are all false in the foundation.
- Production Rust has complete statement, function, region, and branch coverage targets.
- Public Rust API documentation is complete.
- All action dependencies are pinned by full commit SHA.
