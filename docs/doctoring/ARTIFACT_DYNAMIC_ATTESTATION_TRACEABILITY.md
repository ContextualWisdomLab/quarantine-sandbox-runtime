# Artifact Dynamic Attestation Traceability

Status: Proposed evidence-contract boundary; issue #52; production behavior unchanged.

## Problem

The public artifact-analysis contract already exposes `AnalysisProfile::LinuxDynamic`, `AnalysisProfile::WindowsDynamic`, `EvidenceKind::RuntimeBehavior`, and `RuntimeManifest.dynamic_execution_performed`. The product requirements describe approved analysis profiles as executable under quarantine, while the current technical contract correctly returns `Inconclusive` when a requested dynamic worker is unavailable.

`RuntimeManifest::validate()` nevertheless rejects every manifest where `dynamic_execution_performed=true`, regardless of `requested_profile`. Because `EvidenceBundle::validate()` always invokes that validator, the current `1.0.0` Rust contract cannot represent a truthful completed dynamic analysis. A future isolated worker would have to emit `false` and understate execution, or emit `true` and make its evidence bundle invalid.

This is independent from issue #49, which owns analyzer/worker capability isolation, and issue #50, which owns bounded worker-to-controller result ingestion. Issue #52 owns the semantics of the evidence receipt once approved dynamic execution actually occurs.

## Constraints

- `StaticOnly` remains a non-executing contract; `dynamic_execution_performed=true` is invalid for that profile.
- An unavailable dynamic worker remains `Inconclusive` and must not be represented as completed execution.
- Current deny-by-default profiles do not gain network access or credentials merely because dynamic execution becomes representable.
- `RuntimeBehavior` evidence must be attributable to the exact artifact, analysis profile, worker invocation, policy, and immutable runtime authority before a completed dynamic receipt is trusted.
- A boolean alone is not execution or isolation evidence. Issue #49 worker containment, issue #50 result-channel bounds, runtime cleanup, and real backend evidence remain independent release gates.
- If changing `1.0.0` validation would change an established wire meaning, version the contract and JSON Schema rather than silently redefining it.

## Alternatives

### Keep all runtime-manifest execution booleans permanently false

Rejected. That preserves the static foundation but makes the already-published dynamic profile and `RuntimeBehavior` vocabulary unable to report actual dynamic execution truthfully.

### Allow `dynamic_execution_performed=true` for every profile

Rejected. This would weaken the static-only invariant and permit a static receipt to claim execution.

### Infer execution only from the presence of `RuntimeBehavior`

Rejected as insufficient. Evidence-kind presence does not replace an explicit runtime execution fact, and malformed or forged bundles still need cross-field validation.

### Make manifest validation profile-aware and bind it to exact worker evidence

Selected direction after causal RED. Static-only receipts require no execution; completed dynamic receipts may represent actual execution only when attributable isolated-worker evidence exists. Unavailable dynamic execution remains incomplete rather than being silently downgraded or fabricated.

## RED

Test-bearing authority: `4cc901d7cb40bdc833e08b0c695ba12c27fa2f68`, `tests/artifact_analysis_dynamic_attestation_red.rs` (issue #52).

The regression constructs otherwise-valid completed evidence bundles for `LinuxDynamic` and `WindowsDynamic` with `dynamic_execution_performed=true`, `network_access_performed=false`, `credentials_available=false`, and attributable `RuntimeBehavior` evidence. Both must be representable by the contract. A companion assertion requires `StaticOnly + dynamic_execution_performed=true` to remain `RuntimeBoundaryViolated`.

Current production is expected to RED because `RuntimeManifest::validate()` unconditionally treats `dynamic_execution_performed=true` as a foundation boundary violation.

## Smallest causal GREEN after executed RED

Make runtime-manifest/bundle validation profile-aware without implementing or pretending to implement the worker itself. Preserve these semantic states explicitly:

1. `StaticOnly`: dynamic execution is false.
2. Requested dynamic profile with unavailable worker: `Inconclusive`, dynamic execution false.
3. Completed dynamic profile: dynamic execution true only when exact attributable worker/runtime evidence supports it.
4. Current P0 profile: network access and credentials remain false unless a separately versioned policy and evidence model deliberately changes that boundary.

The smallest contract repair must keep Rust types, JSON Schema, PRD/TRD, compatibility tests, and consumer documentation semantically aligned. It must not mark ADR-0009 Accepted or authorize release before issues #49 and #50 plus real worker isolation/resource/cleanup evidence are GREEN on one unchanged integrated candidate.

## Release evidence

A dynamic artifact-analysis release requires more than a contract-valid boolean. The exact worker invocation must be bound to immutable artifact/profile/analyzer/runtime identity, capability-denying isolation, bounded CPU/RAM/PID/time/storage/output, bounded result ingestion, deterministic failure attribution, leak-free termination/cleanup, current-head security/coverage/SBOM/provenance, and protected integration. Static evidence must remain distinguishable from observed runtime behavior.

## References

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
