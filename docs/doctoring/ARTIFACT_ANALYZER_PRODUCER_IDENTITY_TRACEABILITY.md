# Artifact Analyzer Producer Identity Traceability

## Decision scope

This traceability note covers artifact-analysis producer identity admission on parent PR #18 exact `c0647152ec052d82969b2ae078891e25e6d4d69a` and Issue #99. It does not change analyzer isolation, evidence taxonomy, Wardnet verdict authority, or consumer authorization authority.

## Observed defect

`AnalysisEngine::validate_configuration` currently accepts every syntactically valid, non-duplicate analyzer identifier. `runtime_core` satisfies that syntax and is not reserved. The same runtime emits Core-owned artifact-identity and policy-boundary evidence with `producer_id = "runtime_core"`. An externally supplied analyzer can therefore claim the same producer identifier even though it does not hold Core evidence authority.

The defect is an acceptance-set and provenance-authority collision: producer identity is metadata used to distinguish evidence origin, but analyzer admission currently accepts a value reserved in practice by the runtime itself.

## Causal RED

Test-only commit `286e54c6898be0ac347f582d2376f942dc3e8aeb` adds `tests/artifact_analysis_reserved_producer_id_red.rs`. It requires an otherwise-valid analyzer whose `analyzer_id()` is `runtime_core` to fail during `AnalysisEngine::new(...)` with the existing `AnalysisError::InvalidAnalyzerIdentifier` contract. The analyzer body panics if invoked, proving the acceptance boundary belongs before analyzer execution.

No production code is changed by that commit. It is checked-in RED only until exact-head CI executes and fails for the reserved-identity cause.

## Minimum repair after causal execution

Reserve `runtime_core` in analyzer admission. Preserve:

- the Core-owned producer identifier on Core evidence;
- existing syntax and duplicate-identifier validation;
- the public error surface unless executed evidence requires a distinct variant;
- fail-closed external-analyzer isolation and the #49/#69/#70 worker stack;
- evidence schemas and consumer authority boundaries.

Rejected alternatives are renaming individual Core records, silently rewriting analyzer identifiers, post-hoc namespacing after admission, and allowing consumers to infer producer trust from evidence kind. Those approaches leave an ambiguous producer namespace or mutate untrusted input instead of rejecting it at the owner boundary.

## Security and DDD rationale

Within the `artifact_analysis` Supporting bounded context, producer identity is part of the evidence Ubiquitous Language. A runtime-owned producer namespace must not be assignable to externally supplied analyzers. NIST SI-10 requires validating system inputs against their defined syntax, semantics, and acceptable values; `runtime_core` is syntactically valid but semantically outside the analyzer acceptance set. MITRE CWE-345 describes failures to verify the origin or authenticity of data; this issue is tracked as an engineering integrity analogy rather than a CVE mapping because CWE marks the class as discouraged for vulnerability mapping.

## Verification and integration gate

RED → minimum admission fix → exact-head `cargo fmt --check` → workspace tests → Clippy `-D warnings` → rustdoc `-D warnings` → repository validation → exact 100% owned-production statement/function/region/branch coverage → qualifying review/security gates → dependency-safe non-force adoption by #18. Dedicated positive effective-LSM remains an independent release gate.

## References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Revision 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

MITRE. (2026). *CWE-345: Insufficient verification of data authenticity* (CWE 4.20). Common Weakness Enumeration. https://cwe.mitre.org/data/definitions/345.html
