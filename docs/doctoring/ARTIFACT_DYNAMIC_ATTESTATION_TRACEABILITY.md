# Artifact Dynamic Attestation Traceability

Status: Proposed evidence-contract boundary; issue #52. The original dynamic-attestation RED executed on `9d0da2a50e3247fd6cd2e52301d57408d404557d` / CI `35342943374`. The hardened inverse-evidence RED then executed on `78f721afbf269a00721c82c155d9686d0309883f` / CI `35509858228` and failed for the intended missing relation. Review `5289608220` authorizes only the minimum matching Rust and Draft 2020-12 schema repair. That repair is checked in, but the repaired exact head still requires fresh CI; no GREEN, merge, or release claim is made here.

## Problem

The public artifact-analysis contract exposes `AnalysisProfile::{LinuxDynamic, WindowsDynamic}`, `EvidenceKind::RuntimeBehavior`, and `RuntimeManifest.dynamic_execution_performed`. Approved dynamic analysis must be representable truthfully, while an unavailable dynamic worker must remain incomplete and fail closed.

Exact `9d0da2a50e3247fd6cd2e52301d57408d404557d` / CI `35342943374` passed exact checkout, dependency lock, repository policy, coverage-parser checks, and Rust 1.97.1 rustfmt before the focused regression executed. Five failures established the original receipt defect:

- truthful completed Linux/Windows dynamic execution with `dynamic_execution_performed=true` was rejected;
- dynamic `execution=false` could still validate as `Completed`;
- `StaticOnly + execution=false` could retain `RuntimeBehavior`;
- unavailable dynamic `Inconclusive + execution=false` could retain `RuntimeBehavior`;
- the Draft 2020-12 schema could not admit truthful completed dynamic execution or enforce equivalent cross-field rules.

Two controls remained valid in the same run: `StaticOnly + execution=true` was rejected, and unavailable dynamic `Inconclusive + execution=false` without runtime-behavior evidence remained representable. Review `5259590778` records that executed evidence.

Candidate commits `387406c93953e8d8efbe4e32671ec764a2805b00` and `4a9cd5919691cda805ec72131f980f2aef631915` minimally made truthful dynamic completion representable, rejected completed dynamic receipts that claim no execution, and rejected `RuntimeBehavior` whenever execution is false. Network and credential denial remain fail closed.

Review `5260321336` then found that this candidate was still one-directional: `RuntimeBehavior -> dynamic_execution_performed=true` was enforced, but `dynamic_execution_performed=true -> RuntimeBehavior` was not. A completed dynamic receipt could therefore claim execution while carrying no observed runtime-behavior record. A boolean alone is not execution evidence.

Test-only `8b70dd9e2484ebf0a484a549d2829c3ba55fa9f8` added the Rust RED for a completed Linux/Windows dynamic bundle with `execution=true` but no `RuntimeBehavior`. Test-only `5cd2d46af8158f5d3251257b1fe9ef99831fe08e` added the public-schema structural RED requiring the corresponding narrow `runtime_behavior` occurrence relation.

Review `5260530224` found two false-GREEN paths in those inverse witnesses. The Rust test accepted arbitrary `validate().is_err()`, and the schema matcher allowed hidden sibling predicates that could make the rule vacuous. Test-only `1454ebb19583dfb0c178c544661b44d8ebf2c578` therefore requires the schema `if` to contain only `runtime.dynamic_execution_performed=true`, requires the `then` to contain only the direct `evidence.contains(runtime_behavior)` consequence, and includes hostile controls for hidden root/runtime sibling predicates. Test-only `402b6058a113eba42d8438482652bd39b842a46e` binds the Rust RED to exactly `ContractError::RuntimeBoundaryViolated { boundary_name: "dynamic_execution_without_runtime_behavior" }`.

The hardened exact `78f721afbf269a00721c82c155d9686d0309883f` / CI `35509858228` then executed the inverse witness. The coverage lane reached the test and returned `Ok(())` where the exact `dynamic_execution_without_runtime_behavior` boundary was required, establishing the intended causal RED. Hosted rootless/AppArmor negative evidence remained GREEN in that run. The verify lane independently stopped at rustfmt before tests, and positive SELinux was cancelled, so neither is transferable evidence for the repaired head. Review `5289608220` records that execution and authorizes only the bounded inverse-relation repair.

## Contract boundaries

- `StaticOnly` requires `dynamic_execution_performed=false` and must not carry `RuntimeBehavior`.
- An unavailable dynamic worker remains `Inconclusive + dynamic_execution_performed=false` without `RuntimeBehavior`.
- A dynamic receipt with `dynamic_execution_performed=false` cannot validate as `Completed` or carry observed runtime behavior.
- `RuntimeBehavior` requires `dynamic_execution_performed=true`.
- `dynamic_execution_performed=true` requires at least one narrowly selected `RuntimeBehavior` record.
- Current deny-by-default profiles do not gain network access or credentials merely because dynamic execution becomes representable.
- Receipt consistency does not prove worker containment. Issue #49 owns analyzer/worker capability isolation; issue #50 owns bounded worker-to-controller result ingestion.
- Exact artifact, worker invocation, analyzer provenance, runtime identity, resource bounds, termination, cleanup, and positive effective-isolation evidence remain independent release obligations.
- Rust validation and the public JSON Schema must describe one semantics. If immutable `1.0.0` authority exists before this semantic expansion is released, publish a new version rather than mutating an immutable meaning.

## Alternatives and decision

Keeping `dynamic_execution_performed` permanently false was rejected because it makes the modeled dynamic profiles unable to report real execution. Allowing execution for every profile was rejected because it weakens `StaticOnly`. Leaving completeness to consumers was rejected because `EvidenceBundle::validate()` is the runtime-owned wire-integrity boundary. Treating `RuntimeBehavior` as independent descriptive evidence was rejected because observed behavior contradicts an execution=false manifest. Rust-only repair was rejected because schema-only consumers would see a different contract.

The selected contract is profile-aware bidirectional consistency: static receipts do not execute; unavailable dynamic receipts remain incomplete; completed dynamic receipts require actual execution; observed runtime behavior cannot exist without execution; and an execution=true claim cannot stand alone without observed runtime-behavior evidence.

## RED and repair evidence

Initial truthful-execution authority: `4cc901d7cb40bdc833e08b0c695ba12c27fa2f68`, `tests/artifact_analysis_dynamic_attestation_red.rs`.

Rust cross-field hardening authority: `7fa9116a993d6854ad579db2512ff921edcb8611`.

Initial wire-schema hardening authority: `bb1001016f3efd3183fb903cdde242f0859279e5`.

Executable schema-semantics hardening authority: `e0ddf5abf96ff50718491ec4c59bb3728a71eed9`.

Observed-runtime consistency hardening authority: `2b540a0a2c24c7e53c183ff2a8ea1e85cd30daa8`.

Formatter-clean original executed authority: `9d0da2a50e3247fd6cd2e52301d57408d404557d`, CI `35342943374`, review `5259590778`.

Initial inverse Rust RED authority: `8b70dd9e2484ebf0a484a549d2829c3ba55fa9f8`.

Initial inverse schema RED authority: `5cd2d46af8158f5d3251257b1fe9ef99831fe08e`, review `5260321336`.

Hardened schema inverse RED authority: `1454ebb19583dfb0c178c544661b44d8ebf2c578`, review `5260530224`.

Hardened exact-error Rust inverse RED authority: `402b6058a113eba42d8438482652bd39b842a46e`, review `5260530224`.

Executed hardened inverse authority: `78f721afbf269a00721c82c155d9686d0309883f`, CI `35509858228`, review `5289608220`. The coverage lane causally observed `Ok(())` instead of `RuntimeBoundaryViolated { boundary_name: "dynamic_execution_without_runtime_behavior" }`.

Minimum Rust repair: `da376ef8c8e4b900a0a42d654a99129c53a89bcd`, adding only the inverse `dynamic_execution_performed=true -> RuntimeBehavior` validation with the exact boundary name.

Minimum public-schema repair: `593f5610bf9bbd876e42cfaa02993d7df0cbde7f`, adding only the exact execution=true trigger and direct `evidence.contains(evidence_kind=runtime_behavior)` consequence.

Formatting repairs required by the executed verify evidence: `7eaf6e4e189ae34d2d51a3f03afd3e03d6a10a1b`, `2a51056a6e6cf0edb872d4fb52628b0d9156a828`, and `adc2fedaadcc6bc60662c685dc39d9c49df64d46`. These do not widen runtime semantics.

No predecessor success transfers to the repaired head. Fresh exact-head CI must re-run repository policy, rustfmt, tests, Clippy/rustdoc, owned-production coverage, hosted negative evidence, and positive effective-LSM evidence before merge or publication.

## Minimum GREEN condition

The bounded repair is complete only when a fresh exact head proves all of the following together:

1. Rust `EvidenceBundle::validate()` rejects `dynamic_execution_performed=true` with no `RuntimeBehavior` as exactly `RuntimeBoundaryViolated { boundary_name: "dynamic_execution_without_runtime_behavior" }`.
2. The Draft 2020-12 schema enforces the same relation with a direct narrow `contains` selector for `evidence_kind=runtime_behavior`; the trigger and consequence contain no hidden sibling predicates.
3. Existing static-only, unavailable-dynamic, completed-dynamic, network-denial, and credential-denial semantics remain unchanged.
4. Hostile input is rejected rather than normalized or inferred.
5. Repository formatting, lint, documentation, exact-head coverage, and applicable runtime-isolation evidence are GREEN on the same head.

Worker isolation, result-channel logic, provider policy, network widening, credential widening, and verdict ownership remain out of scope for this lane.

## Release evidence

Dynamic artifact-analysis release readiness requires more than a contract-valid boolean or matching Rust values. The exact worker invocation must be bound to immutable artifact/profile/analyzer/runtime identity, capability-denying isolation, bounded CPU/RAM/PID/time/storage/output, bounded result ingestion, deterministic failure attribution, leak-free termination/cleanup, exact-head security and coverage, SBOM/provenance, protected integration, and positive effective-LSM/runtime evidence. Unavailable dynamic execution must never be upgraded to `Completed`, and schema/Rust consumers must reject the same contradictory states.

## References

JSON Schema. (2022). *JSON Schema core: A media type for describing JSON documents, Draft 2020-12*. https://json-schema.org/draft/2020-12/json-schema-core

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
