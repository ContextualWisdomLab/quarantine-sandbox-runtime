# ADR 0012: Offline artifact-binding verification

- **Status:** Proposed
- **Date:** 2026-10-06

This ADR remains Proposed while its implementation exists only on a Draft PR stacked on root Draft #1. Promote it to Accepted only after protected integration and current-head verification.

## Context

`docs/PRD.md` promises a security consumer immutable artifact identity plus attributable evidence, and auditors who can verify what was analyzed. The runtime computes the artifact SHA-256 digest, byte length and non-executing format classification exactly once, inside `ingest_bytes`. `EvidenceBundle::validate` checks only the shape of those fields: any 64-character lower-case hexadecimal digest, any non-zero size and any `ArtifactKind` pass. The consumer still holds the original bytes (see `docs/contracts/consumer-contract.md`), but has no runtime-owned way to confirm that a serialized bundle describes those bytes. A bundle that was edited, attached to the wrong sample, or replayed for a different artifact is accepted by every public check.

ADR numbers 0007–0011 are used by other unmerged lineages (#14, #18, #150, #151), so this decision uses 0012.

## Decision

Add one pure, synchronous library function in the `artifact_analysis` Supporting context:

```rust
pub fn verify_artifact_binding(
    bundle: &EvidenceBundle,
    artifact_bytes: &[u8],
) -> Result<ArtifactBindingReport, ArtifactBindingError>;
```

- The function first runs `EvidenceBundle::validate`. A contract violation returns `ArtifactBindingError::Contract` before any digest is compared.
- Empty `artifact_bytes` return `ArtifactBindingError::EmptyArtifact`, because ingestion never produces a bundle for empty input.
- Otherwise it recomputes three values from the supplied bytes with the same derivation as ingestion: the lower-case SHA-256 hexadecimal digest, the byte length, and the non-executing `ArtifactKind` classification using the bundle's `original_file_name` (or no name) exactly as ingestion does.
- It returns an `ArtifactBindingReport` with one `FieldBinding` (`matched` or `mismatched`) for each of `artifact_sha256`, `artifact_size_bytes` and `artifact_kind`, the three recomputed values, and an overall `ArtifactBindingOutcome` that is `matched` only when all three fields match.
- A mismatch is a report, not an error. The caller decides what to do with it.
- `ArtifactBindingReport` has private fields, read-only accessors and `Serialize` only. It has no `Deserialize`, so a caller cannot construct a report that claims a match.
- The function reads no files, executes no artifact bytes, uses no network and does not allocate a copy of the artifact.

Public names, re-exported at the crate root:

| Name | Kind | Wire values |
| --- | --- | --- |
| `verify_artifact_binding` | function | — |
| `ArtifactBindingReport` | struct | `outcome`, `artifact_sha256`, `artifact_size_bytes`, `artifact_kind`, `computed_artifact_sha256`, `computed_artifact_size_bytes`, `computed_artifact_kind` |
| `ArtifactBindingOutcome` | enum | `matched`, `mismatched` |
| `FieldBinding` | enum | `matched`, `mismatched` |
| `ArtifactBindingError` | enum | `Contract(ContractError)`, `EmptyArtifact` |

## Non-claims

- A `matched` outcome is not a verdict and not a completeness statement. `consumer_verdict_required` stays true and the consumer still reads `disposition` and `limitations`.
- It is not authenticity. Anyone who can edit both the bundle and the bytes can still produce agreement. Signed provenance remains release work.
- It does not bind the `artifact_identity` evidence record to the top-level descriptor (#58), recompute `analysis_job_id` (#66), reject unknown bundle fields (#75) or add any other `EvidenceBundle` invariant. It inherits those checks through `EvidenceBundle::validate` once they land.
- Static classification is not observed runtime behavior.

## Consequences

- A Rust consumer or a later transport can confirm that a bundle describes the bytes it holds without re-running analysis.
- A command-line transport for this function is deferred until the bounded regular-file reader proposed in #150 is integrated, so the reader is not duplicated.
- The classifier `detect_artifact_kind` becomes visible to its parent module (`pub(super)`). Its rules do not change, and a property test binds the verifier's derivation to `AnalysisEngine` output.
