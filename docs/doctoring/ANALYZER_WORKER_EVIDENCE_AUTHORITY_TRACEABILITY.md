# Analyzer Worker Evidence Authority Traceability

Issue #77 defines the controller-side authority boundary for evidence kinds returned by an isolated analyzer worker. It is a focused descendant of Draft #70 and does not change Core `sandbox_execution` isolation/resource/lifecycle ownership.

## Current authority

- Parent worker-port authority: Draft #70 exact `c7a05a3870164c8811790af5f4d81d46dc7dc43b`.
- Parent DDD decision: ADR-0009 remains Proposed; `artifact_analysis` owns analyzer/result semantics, `sandbox_execution` owns reusable isolation/resource/lifecycle semantics, and `infrastructure` owns concrete enforcement and observation.
- RED authority: `tests/artifact_analysis_worker_evidence_authority_red.rs`, introduced by `3b6693394a89030c286839145cbdda6aa0a83bad`.

The current worker finding contract carries the repository-wide `EvidenceKind` and `AnalyzerWorkerReceipt::validate_against` validates only bounded text/attributes plus request and Core isolation consistency. It does not constrain which evidence authorities an untrusted analyzer may claim. As a result, a worker can currently label one normalized finding as `ArtifactIdentity` or `PolicyBoundary` even though those facts are assembled from controller-admitted immutable bytes and runtime-owned boundary state.

## Decision

Treat evidence kind as an authority-bearing field, not merely an enum-shaped label. The controller-owned worker-result ACL must reject analyzer claims for evidence categories whose truth source belongs to the controller or runtime.

The first minimum boundary is deliberately narrow:

- `ArtifactIdentity` is controller-owned because immutable subject identity comes from admitted artifact bytes.
- `PolicyBoundary` is runtime/controller-owned because isolation and policy-boundary truth comes from runtime-observed state rather than analyzer self-report.
- `FileFormat` remains admissible because the product already has analyzer-produced non-executing format classification.
- `StaticCapability` remains analyzer-owned.
- `RuntimeBehavior` and `NetworkAttempt` are not made generally admissible by this issue; issue #52/#53 owns dynamic-execution truthfulness and completeness.
- `ToolFailure` remains attributable failure evidence and is not redefined here.

Forbidden kinds must fail closed. Do not rewrite them into another kind, drop them silently, or infer authorization from a matching worker receipt.

## RED

`worker_cannot_claim_controller_or_runtime_owned_foundation_evidence` constructs an otherwise-valid worker request/receipt and independently changes the returned finding kind to `ArtifactIdentity` and `PolicyBoundary`. Each must be rejected as `AnalyzerWorkerContractError::InvalidOutcome { field_name: "evidence_kind" }`.

`worker_owned_static_evidence_remains_admissible` protects the intended existing analyzer path by requiring `FileFormat` and `StaticCapability` to remain valid under the same request/isolation fixture.

This is checked-in RED only until an unchanged exact head executes it for the authority-gap cause. The minimum GREEN is a controller-owned evidence-authority predicate in `artifact_analysis::analyzer_worker` used by outcome validation. It must not move evidence semantics into Core, add backend-specific types, or expand dynamic execution authority.

## Security rationale

NIST SP 800-53 Rev. 5.1 control SI-10 requires systems to validate information inputs against defined syntax, semantics, and acceptable values. For this boundary, the acceptable value set is context-sensitive: an enum value can be syntactically valid while semantically unauthorized for an untrusted analyzer producer. The controller therefore validates both the shape and the authority of worker-supplied evidence metadata.

Saltzer and Schroeder's complete-mediation and fail-safe-default principles support the same boundary: authority should be checked at the point where an untrusted result crosses into trusted controller state, and absence of an explicit authorization rule must not become implicit permission.

## Alternatives rejected

- Accept every `EvidenceKind` because it is a public enum: rejected because type membership does not establish producer authority.
- Infer authority from `producer_id`: rejected because producer display identity is not the trust root for foundation/runtime facts.
- Rewrite forbidden kinds into `StaticCapability` or `ToolFailure`: rejected because silent coercion destroys provenance and can turn a security contradiction into apparently valid evidence.
- Move the enum or authority table into `sandbox_execution`: rejected because evidence semantics belong to Supporting `artifact_analysis`; Core owns isolation/resource/lifecycle truth, not analyzer evidence taxonomy.
- Solve `RuntimeBehavior` / `NetworkAttempt` in this slice: rejected because #52/#53 already owns the dynamic-execution/completeness contract and combining it would blur the causal RED.

## References

Joint Task Force. (2020/2025). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5.1), SI-10 Information Input Validation. National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Saltzer, J. H., & Schroeder, M. D. (1975). The protection of information in computer systems. *Proceedings of the IEEE, 63*(9), 1278–1308. https://doi.org/10.1109/PROC.1975.9939

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
