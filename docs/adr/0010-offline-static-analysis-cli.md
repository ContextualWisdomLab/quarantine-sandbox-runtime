# ADR 0010: Offline static artifact-analysis CLI transport

- **Status:** Proposed
- **Date:** 2026-10-06

This ADR remains Proposed while its implementation exists only on a Draft PR stacked on root Draft #1. Promote it to Accepted only after protected integration and current-head verification.

## Context

`docs/PRD.md` requires that a security consumer can submit bytes without source credentials and receive immutable artifact identity plus attributable evidence. The static foundation implements this as the Rust library API `AnalysisEngine::analyze_bytes`. No runnable transport exists, so an operator or CI job cannot produce a published `EvidenceBundle` from a file without writing Rust code (issue #148).

The only proposed CLI elsewhere (ADR-0008 on Draft #14) is a Podman command-execution transport. It depends on runtime-gate, positive-LSM and release gates and does not cover static artifact analysis. ADR numbers 0007–0009 are used by other unmerged lineages, so this decision uses 0010 to avoid a future collision.

## Decision

Add one offline, synchronous binary `qsr-analyze` that uses only the standard library plus `libc` open-flag constants:

```text
qsr-analyze --request <analysis-request.json> --artifact <path>
```

- The binary is a thin shim. Testable logic lives in `src/artifact_analysis/cli.rs`, an inbound adapter of the `artifact_analysis` Supporting context. It owns no new domain vocabulary.
- It reads a closed `AnalysisRequest` 1.0.0 document of at most 64 KiB and validates it through the existing contract (`deny_unknown_fields` plus `validate`).
- It reads artifact bytes from one regular file, bounded by `IngestionPolicy::default().maximum_artifact_bytes` plus one byte. Symbolic links (final component), directories, FIFOs and devices are rejected before analysis. The open uses `O_NONBLOCK` so a FIFO cannot block the process.
- It runs only `AnalysisEngine::default()`, which uses the non-executing `FormatAnalyzer`. Artifact bytes are never executed. Dynamic profiles remain `inconclusive` with `dynamic_analysis_not_configured`.
- Exit `0` means a validated bundle was written to stdout. It is not a verdict; the consumer still reads `disposition`, and `consumer_verdict_required` stays true.
- Failures write one fixed line `qsr-analyze: error=<code>` to stderr and never echo request bodies, artifact bytes or host paths.

| Exit | Code | Meaning |
| --- | --- | --- |
| 0 | — | Bundle emitted |
| 64 | `usage` | Missing, duplicate or unknown arguments |
| 65 | `invalid_request` | Unreadable, oversized, malformed or invalid request |
| 66 | `artifact_unavailable` | Artifact missing, not a regular file, or unreadable |
| 67 | `artifact_rejected` | Artifact over the byte bound or rejected by ingestion |
| 70 | `internal` | Engine, output serialization or write failure |

No-follow opening uses `O_NOFOLLOW | O_NONBLOCK | O_NOCTTY` from the `libc` crate, already present in the lock graph, through safe `OpenOptionsExt::custom_flags`; no `unsafe` code is added. The file type is then checked with `fstat` on the open descriptor, so the check and the read use the same object. Devices are opened and then rejected before any read. Opening some device nodes can itself have side effects (for example, a tape rewind), which is a further reason the paths must be trusted; `O_NOCTTY` prevents a terminal device from becoming the controlling terminal. Only the final path component is protected, and hard links are accepted, so paths must come from a trusted operator. The transport is Unix-only in this increment. Non-Unix targets compile but are not supported or tested: the request open always fails, so every run exits 65 `invalid_request`.

On any nonzero exit, consumers must discard stdout. A write failure (exit 70) can leave truncated JSON on stdout.

## Consequences

- Operators and CI can produce static evidence without a daemon, network, credentials or container backend.
- The CLI adds no dynamic analysis, Podman execution, external analyzer adapters, persistence or verdict authority.
- A future HTTP or queue transport must reuse the same inbound adapter semantics or record a new ADR.
