# Artifact Dynamic Attestation Traceability

Status: Proposed evidence-contract boundary; issue #52; production behavior unchanged.

## Problem

The public artifact-analysis contract already exposes `AnalysisProfile::LinuxDynamic`, `AnalysisProfile::WindowsDynamic`, `EvidenceKind::RuntimeBehavior`, and `RuntimeManifest.dynamic_execution_performed`. The product requirements describe approved analysis profiles as executable under quarantine, while the current technical contract correctly returns `Inconclusive` when a requested dynamic worker is unavailable.

`RuntimeManifest::validate()` nevertheless rejects every manifest where `dynamic_execution_performed=true`, regardless of `requested_profile`. Because `EvidenceBundle::validate()` always invokes that validator, the current `1.0.0` Rust contract cannot represent a truthful completed dynamic analysis. A future isolated worker would have to emit `false` and understate execution, or emit `true` and make its evidence bundle invalid.

A second cross-field gap is equally important: the current bundle validator does not bind `requested_profile`, `dynamic_execution_performed`, and `disposition` together. A hand-constructed or deserialized `LinuxDynamic`/`WindowsDynamic` bundle can currently set `dynamic_execution_performed=false` and still validate with `RuntimeDisposition::Completed`. That would let a missing worker be presented as complete evidence even though the TRD requires unavailable dynamic profiles to fail closed as incomplete.

A third gap is the reverse evidence-consistency direction. `EvidenceBundle::validate()` does not bind `EvidenceKind::RuntimeBehavior` to a dynamic profile whose execution flag is true. A reconstructed `StaticOnly` receipt can set `dynamic_execution_performed=false` yet carry purported observed runtime behavior, and an unavailable dynamic receipt can remain `Inconclusive + execution=false` while retaining `RuntimeBehavior`. Both states manufacture runtime observation without an execution boundary and contradict the repository invariant that static evidence is never labeled observed runtime behavior.

The current `schemas/evidence-bundle.schema.json` duplicates these defects at the wire boundary: `dynamic_execution_performed` is globally fixed to `false`, while `requested_profile`, execution, `disposition`, and evidence kinds have no executable cross-field consistency rule. Repairing Rust validation without repairing the schema would leave a second consumer-visible contract with different semantics.

This is independent from issue #49, which owns analyzer/worker capability isolation, and issue #50, which owns bounded worker-to-controller result ingestion. Issue #52 owns the semantics of the evidence receipt once approved dynamic execution actually occurs or is unavailable.

## Constraints

- `StaticOnly` remains a non-executing contract; `dynamic_execution_performed=true` is invalid for that profile.
- `StaticOnly` must not carry `RuntimeBehavior` evidence even if its execution flag is false.
- An unavailable dynamic worker remains `Inconclusive` with `dynamic_execution_performed=false`; the same state must not validate as `Completed` or retain `RuntimeBehavior` evidence.
- `RuntimeBehavior` requires a requested dynamic profile, actual dynamic execution, and attribution to the exact artifact, worker invocation, policy, and immutable runtime authority.
- Current deny-by-default profiles do not gain network access or credentials merely because dynamic execution becomes representable.
- A boolean alone is not execution or isolation evidence. Issue #49 worker containment, issue #50 result-channel bounds, runtime cleanup, and real backend evidence remain independent release gates.
- Rust validation and JSON Schema must enforce the same profile/execution/completeness/evidence-kind semantics. JSON Schema Draft 2020-12 provides conditional, array, and combinator applicators for cross-field assertions.
- If changing `1.0.0` validation would change an established wire meaning, version the contract and JSON Schema rather than silently redefining it.

## Alternatives

### Keep all runtime-manifest execution booleans permanently false

Rejected. That preserves the static foundation but makes the already-published dynamic profile and `RuntimeBehavior` vocabulary unable to report actual dynamic execution truthfully.

### Allow `dynamic_execution_performed=true` for every profile

Rejected. This would weaken the static-only invariant and permit a static receipt to claim execution.

### Validate runtime fields independently and leave completeness to consumers

Rejected. `EvidenceBundle::validate()` is the runtime's own wire-integrity boundary. Accepting `requested_profile=linux_dynamic`, `dynamic_execution_performed=false`, and `disposition=completed` makes a semantically incomplete receipt structurally valid and pushes a security-relevant invariant onto every consumer.

### Treat `RuntimeBehavior` as independent descriptive evidence

Rejected. `RuntimeBehavior` is explicitly observed behavior from a dynamic worker. Allowing it when execution is false or the profile is static would make a receipt claim an observation that its own runtime manifest says did not occur.

### Repair only the Rust validator

Rejected. The JSON Schema is a published compatibility surface. Leaving `const:false` or independent completeness/evidence fields in the schema would make Rust and wire validation disagree.

### Infer execution only from the presence of `RuntimeBehavior`

Rejected as insufficient. Evidence-kind presence does not replace an explicit runtime execution fact, and malformed or forged bundles still need cross-field validation. The required relation is consistency in both directions, not inference from one field alone.

### Make manifest/bundle validation profile-aware and bind it to exact worker evidence

Selected direction after causal RED. Static-only receipts require no execution and no runtime-behavior evidence; unavailable dynamic receipts remain incomplete and contain no observed runtime behavior; completed dynamic receipts may represent actual execution only when attributable isolated-worker evidence exists. The selected wire schema must encode the same rule rather than validating each field in isolation.

## RED

Initial truthful-execution authority: `4cc901d7cb40bdc833e08b0c695ba12c27fa2f68`, `tests/artifact_analysis_dynamic_attestation_red.rs` (issue #52).

Rust cross-field hardening authority: `7fa9116a993d6854ad579db2512ff921edcb8611`.

Initial wire-schema hardening authority: `bb1001016f3efd3183fb903cdde242f0859279e5`.

Executable schema-semantics hardening authority: `e0ddf5abf96ff50718491ec4c59bb3728a71eed9`.

Observed-runtime consistency hardening authority: `2b540a0a2c24c7e53c183ff2a8ea1e85cd30daa8`.

The regression now covers the following semantic boundaries:

1. otherwise-valid completed `LinuxDynamic` and `WindowsDynamic` bundles with `dynamic_execution_performed=true`, no network/credentials, and attributable `RuntimeBehavior` evidence must be representable;
2. unavailable Linux/Windows dynamic profiles with `dynamic_execution_performed=false` and `Inconclusive` remain valid receipts when they contain no observed runtime behavior;
3. the same unavailable dynamic state must not validate after changing only `disposition` to `Completed`;
4. `StaticOnly + dynamic_execution_performed=true` remains invalid;
5. `StaticOnly + dynamic_execution_performed=false + RuntimeBehavior` must be rejected;
6. unavailable dynamic `Inconclusive + dynamic_execution_performed=false + RuntimeBehavior` must be rejected;
7. the evidence-bundle JSON Schema must not globally force `dynamic_execution_performed=false` once approved dynamic completion is representable;
8. the schema must execute equivalent cross-field rules over representative serialized receipts, including evidence-kind consistency, while leaving the exact Draft 2020-12 composition strategy open.

Current production/contract is expected to RED in four independent ways: truthful completed dynamic bundles fail because `RuntimeManifest::validate()` unconditionally rejects execution; false-completion bundles currently validate because `EvidenceBundle::validate()` does not cross-bind requested profile, actual execution, and completeness; static or unavailable-dynamic bundles can retain `RuntimeBehavior` despite execution=false because evidence kinds are not cross-bound; and the checked-in 1.0.0 schema globally forbids execution while lacking the equivalent semantic guards.

## Smallest causal GREEN after executed RED

Make runtime-manifest/bundle validation profile-aware without implementing or pretending to implement the worker itself. Preserve these semantic states explicitly:

1. `StaticOnly`: dynamic execution is false and `RuntimeBehavior` is absent.
2. Requested dynamic profile with unavailable worker: `Inconclusive`, dynamic execution false, and no `RuntimeBehavior` evidence.
3. Completed dynamic profile: dynamic execution true only when exact attributable worker/runtime evidence supports it.
4. A dynamic profile with execution false cannot validate as `Completed` or carry observed runtime behavior.
5. Current P0 profile: network access and credentials remain false unless a separately versioned policy and evidence model deliberately changes that boundary.

The smallest contract repair must keep Rust types, JSON Schema, PRD/TRD, compatibility tests, and consumer documentation semantically aligned. JSON Schema Draft 2020-12 conditional/combinator and array applicators are available for cross-field constraints; schema structure remains an implementation choice, but the wire contract must reject the same false-completion and ghost-runtime-behavior states as Rust validation and must not retain an unconditional execution=false constraint. The repair must not mark ADR-0009 Accepted or authorize release before issues #49 and #50 plus real worker isolation/resource/cleanup evidence are GREEN on one unchanged integrated candidate.

## Release evidence

A dynamic artifact-analysis release requires more than a contract-valid boolean. The exact worker invocation must be bound to immutable artifact/profile/analyzer/runtime identity, capability-denying isolation, bounded CPU/RAM/PID/time/storage/output, bounded result ingestion, deterministic failure attribution, leak-free termination/cleanup, current-head security/coverage/SBOM/provenance, and protected integration. Static evidence must remain distinguishable from observed runtime behavior; an unavailable dynamic worker must never be upgraded from `Inconclusive` to `Completed` or retain fabricated `RuntimeBehavior` through serialization/consumer reconstruction; and Rust/schema validators must agree on those states.

## References

JSON Schema. (2022). *JSON Schema core: A media type for describing JSON documents, Draft 2020-12*. https://json-schema.org/draft/2020-12/json-schema-core

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
