# Artifact Analyzer Producer Identity Traceability

## Decision scope

This note covers artifact-analysis producer identity admission on current parent PR #18 exact `703b4b1a047321bb08d1d6cb1de78d01cb350696` and Issue #99. It does not change analyzer isolation, evidence taxonomy, Wardnet verdict authority, or consumer authorization authority.

Repository-wide `docs/product-technical-gap-baseline.md` is a single-writer ledger owned by canonical gap-baseline lane #121, not by this focused producer-identity leaf.

## Observed defect

`AnalysisEngine::validate_configuration` previously accepted every syntactically valid, non-duplicate analyzer identifier. `runtime_core` satisfies that syntax. The same runtime emits Core-owned `ArtifactIdentity` and `PolicyBoundary` evidence with `producer_id = "runtime_core"`, so an externally supplied analyzer could claim the same producer identifier without Core evidence authority.

This is an acceptance-set and provenance-authority collision. Producer identity belongs to the `artifact_analysis` Ubiquitous Language, but a runtime-owned namespace must not be assignable to an externally supplied analyzer.

## Executed causal RED

Test-only `286e54c6898be0ac347f582d2376f942dc3e8aeb` adds `tests/artifact_analysis_reserved_producer_id_red.rs`. It requires an otherwise-valid analyzer whose `analyzer_id()` is `runtime_core` to fail during `AnalysisEngine::new(...)` with the existing `AnalysisError::InvalidAnalyzerIdentifier` boundary. The analyzer body panics if invoked, proving the rejection belongs before analyzer execution.

Gap-ledger descendant `9bd26abc84a454725d9ae4b27d72dc14c6c39ff3` reached native CI `34196213076` after checkout, dependency lock, repository policy, coverage-parser tests, and formatting. The reserved-producer witness supplied causal RED evidence for the missing semantic reservation.

## Minimum repair and focused GREEN

Production `c2a8944e0e243d88cee823503ab012ecaf85cee2` introduces one private canonical `RUNTIME_CORE_PRODUCER_ID = "runtime_core"`, rejects that identifier during analyzer admission through the existing `InvalidAnalyzerIdentifier` surface, and reuses the same constant for the two Core-owned evidence records. Formatter-only `c34b6f74378187d9466f6061293ea1ee74838d8f` changes layout only.

No public schema, evidence shape, analyzer execution path, isolation behavior, or consumer authority changed. Renaming Core records, silently rewriting analyzer identifiers, post-hoc namespacing, and asking consumers to infer trust from evidence kind were rejected because they preserve ambiguity or mutate untrusted input instead of rejecting it at the owner boundary.

Native CI `34246495133` executed exact `c34b6f74378187d9466f6061293ea1ee74838d8f`. Verify `102129720466` passed checkout, dependency lock, repository policy, coverage-parser tests and formatting; `artifact_analysis_reserved_producer_id_red::engine_rejects_runtime_core_as_analyzer_identifier` passed. The producer reservation is therefore historical focused exact-head GREEN.

The broader workspace later failed in inherited `backend_security_info` command/runtime ancestry. That is a separate process-boundary prerequisite and does not justify weakening producer identity.

## Single-writer gap-ledger repair and current-parent adoption

Review `5229258443` found that #100 still carried a historical `docs/product-technical-gap-baseline.md` delta despite #121 repository-wide ownership. Ordinary fast-forward `5212d0881be40f70a23f02fea0e9330f6c58ded1` restored the global baseline byte-for-byte to the then-current #18 base while preserving the producer-reservation production/test/TRACEABILITY delta.

Parent #18 later advanced from `c0647152ec052d82969b2ae078891e25e6d4d69a` to `703b4b1a047321bb08d1d6cb1de78d01cb350696` with exactly two owner-boundary commits: `docs/doctoring/ARTIFACT_ANALYSIS_GAP_OWNER_REPAIR.md` was added and the repository-wide Gap ledger was restored to its canonical base authority. Ordinary two-parent, non-force adoption `aff7030fbc31dd6c0e21740487fc305157230ecc` takes that parent movement while preserving #100's three owned paths. No source copy, force update, destructive rebase, or predecessor-result transfer is used.

Fresh compare from current #18 to the adoption head contains only `src/artifact_analysis/runtime.rs`, `tests/artifact_analysis_reserved_producer_id_red.rs`, and this TRACEABILITY file. The global Gap ledger is therefore absent from the leaf delta.

## Security and DDD rationale

NIST SI-10 requires validating system inputs against defined syntax, semantics, and acceptable values; `runtime_core` is syntactically valid but semantically outside the external-analyzer acceptance set. MITRE CWE-345 describes failures to verify data origin/authenticity; it is used here as an engineering integrity analogy rather than a vulnerability classification.

## Current gate

The moved head must independently reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, current parent integration, positive effective-isolation evidence where applicable, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication. Historical focused GREEN remains causal evidence only.

## References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Revision 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

MITRE. (2026). *CWE-345: Insufficient verification of data authenticity* (CWE 4.20). Common Weakness Enumeration. https://cwe.mitre.org/data/definitions/345.html
