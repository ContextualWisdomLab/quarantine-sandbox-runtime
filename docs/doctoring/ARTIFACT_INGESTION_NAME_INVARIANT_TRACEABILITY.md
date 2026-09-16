# Artifact ingestion name invariant traceability

Status: focused repair implemented; current branch remains Draft pending moved-head verification.

Authority: artifact-analysis parent `c0647152ec052d82969b2ae078891e25e6d4d69a`, Issue #97, PR #98.

## Problem and DDD invariant

`ArtifactDescriptor` is the canonical artifact identity contract. Its validation bounds artifact names to 255 UTF-8 bytes and treats `original_file_name` as a leaf name: `.`, `..`, control characters, `/`, and `\\` are invalid.

The ingestion anti-corruption layer previously had a wider acceptance set: caller configuration could widen the name limit beyond 255 bytes and ingestion could construct a descriptor from a non-leaf name before applying the canonical descriptor invariant. The bounded-context rule is therefore:

> Every artifact accepted by ingestion must carry an `ArtifactDescriptor` that is valid under the canonical artifact-analysis contract without post-hoc normalization.

A configured ingestion bound may be narrower than the canonical bound, but it may not widen it. Artifact ingestion owns admission of untrusted bytes plus source identity; it does not own path rewriting or consumer-specific normalization.

Repository-wide `docs/product-technical-gap-baseline.md` is a single-writer ledger owned by canonical gap-baseline lane #121, not this focused ingestion leaf.

## Executed RED

Test-only `cfde9d0afe8223352182f74498e14dcc6e14be8c` requires policy bounds above 255 UTF-8 bytes to fail closed, non-leaf names (`.`, `..`, slash/backslash forms) to be rejected, valid Unicode leaf names to remain accepted, and every accepted descriptor to satisfy the canonical contract. Formatter-only `2ce95e647ee65ecebe5b984c6c2b12104cfb17fe` removed the formatting prerequisite.

Native CI `34244806324` on exact `2ce95e647...`, verify `102123949955`, passed checkout, dependency lock, repository validation, coverage-parser tests and formatting before the focused suite executed. The valid Unicode control passed while a 256-byte configured ceiling and `.` were incorrectly accepted. That is causal RED evidence for the acceptance-set mismatch.

## Minimum causal repair and focused GREEN

Production `a860078e661d926cdb084978274f007e62197bb6` constrains `maximum_artifact_name_bytes` to the canonical 255-byte ceiling while preserving narrower consumer limits, then validates the assembled `ArtifactDescriptor` through its canonical domain validator before returning it. Descriptor rejection maps to `IngestionError::InvalidArtifactName`.

No basename extraction, path stripping, truncation, lossy Unicode conversion, silent limit clamp, content mutation, or descriptor-contract widening was introduced.

Native CI `34247172860` executed exact `a860078e...`. Verify `102132037820` passed checkout, dependency lock, repository policy, coverage-parser tests and formatting; all three focused regressions passed: over-wide policy rejection, valid Unicode leaf acceptance/canonical validation, and non-leaf rejection. The ingestion-name repair is therefore historical focused exact-head GREEN.

The broader workspace later failed in inherited `backend_security_info` process/runtime ancestry. That failure belongs to the canonical backend invocation path and does not justify changing the ingestion invariant.

## Single-writer gap-ledger repair

Review `5229248675` found that #98 still carried a historical global Gap delta. Ordinary fast-forward `b55b8ffcc6fd9f87d0e28926458d2e21e42380d7` restores `docs/product-technical-gap-baseline.md` byte-for-byte to exact #18 base blob `ea0310394a3d842246bae380977a30c72c18cbf9`, while preserving the production/test/TRACEABILITY delta. The current documentation head records that owner repair locally. No predecessor execution transfers after head movement.

## Alternatives rejected

- Rely only on a downstream descriptor failure while retaining wider ingestion policy: rejected because the ACL would continue to speak two admission languages.
- Silently clamp configured limits to 255: rejected because it hides invalid operator configuration.
- Strip path components: rejected because it mutates caller identity and can collapse distinct inputs.
- Widen the descriptor contract: rejected because that is a versioned domain-contract decision, not a leaf bug fix.

## Current gate

The moved head must independently reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, parent-safe non-force integration, positive effective-isolation evidence where applicable, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication.

## References

MITRE. (2026). *CWE-20: Improper input validation (Version 4.20).* Common Weakness Enumeration. https://cwe.mitre.org/data/definitions/20.html

National Institute of Standards and Technology. (2020). *Security and privacy controls for information systems and organizations (NIST Special Publication 800-53, Revision 5).* U.S. Department of Commerce. https://doi.org/10.6028/NIST.SP.800-53r5
