# Artifact Dynamic Attestation Traceability

Status: Proposed evidence-contract boundary; issue #52; causal RED executed on `9d0da2a50e3247fd6cd2e52301d57408d404557d`; candidate Rust/wire repair pending exact-head validation.

## Problem

The public artifact-analysis contract exposes `AnalysisProfile::LinuxDynamic`, `AnalysisProfile::WindowsDynamic`, `EvidenceKind::RuntimeBehavior`, and `RuntimeManifest.dynamic_execution_performed`. The product requirements describe approved analysis profiles as executable under quarantine, while the technical contract must continue to return `Inconclusive` when a requested dynamic worker is unavailable.

Exact `9d0da2a50e3247fd6cd2e52301d57408d404557d` / CI `35342943374` executed the hardened regression after exact checkout, dependency lock, repository policy, coverage-parser checks, and Rust 1.97.1 rustfmt had all passed. Five failures established the contract defect rather than a prerequisite failure:

- truthful completed `LinuxDynamic`/`WindowsDynamic` bundles with `dynamic_execution_performed=true` were rejected by `RuntimeManifest::validate()` with `RuntimeBoundaryViolated { boundary_name: "dynamic_execution_performed" }`;
- dynamic bundles with `dynamic_execution_performed=false` could still validate as `Completed`;
- `StaticOnly + execution=false` could retain `RuntimeBehavior`;
- unavailable dynamic `Inconclusive + execution=false` could retain `RuntimeBehavior`;
- the checked-in Draft 2020-12 schema could not admit truthful completed dynamic execution and lacked equivalent cross-field guards.

Two controls remained valid in the same run: `StaticOnly + dynamic_execution_performed=true` was rejected, and an unavailable dynamic profile remained representable as `Inconclusive + execution=false` after observed runtime behavior was removed. Review `5259590778` records the executed finding. Hosted rootless/AppArmor negative evidence was GREEN; the dedicated positive SELinux job received no eligible runner and was cancelled, so this is not release evidence.

The current candidate repair is intentionally limited to receipt semantics. Rust now permits `dynamic_execution_performed=true` only for dynamic profiles, requires a completed dynamic receipt to attest execution, rejects `RuntimeBehavior` whenever execution is false, and continues to reject network or credential use. The public schema now treats the execution field as boolean and applies equivalent conditional/combinator rules for StaticOnly, completed dynamic receipts, and ghost runtime-behavior evidence. This does not implement a worker, prove containment, authorize network access, or satisfy release gates.

This lane remains independent from issue #49, which owns analyzer/worker capability isolation, and issue #50, which owns bounded worker-to-controller result ingestion. Issue #52 owns the semantics of the evidence receipt once approved dynamic execution actually occurs or is unavailable.

## Constraints

- `StaticOnly` remains a non-executing contract; `dynamic_execution_performed=true` is invalid for that profile.
- `StaticOnly` must not carry `RuntimeBehavior` evidence even if its execution flag is false.
- An unavailable dynamic worker remains `Inconclusive` with `dynamic_execution_performed=false`; the same state must not validate as `Completed` or retain `RuntimeBehavior` evidence.
- `RuntimeBehavior` requires actual dynamic execution. Exact artifact, worker invocation, policy, and immutable runtime authority remain separate provenance/release obligations.
- Current deny-by-default profiles do not gain network access or credentials merely because dynamic execution becomes representable.
- A boolean alone is not execution or isolation evidence. Issue #49 worker containment, issue #50 result-channel bounds, runtime cleanup, and real backend evidence remain independent release gates.
- Rust validation and JSON Schema must enforce the same profile/execution/completeness/evidence-kind semantics. JSON Schema Draft 2020-12 provides conditional, array, and combinator applicators for these cross-field assertions.
- If an immutable 1.0.0 authority already exists, semantic expansion must move to a new contract version rather than mutating a released meaning. No immutable QSR release currently exists, but publication/version authority remains governed separately.

## Alternatives

### Keep all runtime-manifest execution booleans permanently false

Rejected. That preserves the static foundation but makes the already-modeled dynamic profile and `RuntimeBehavior` vocabulary unable to report actual dynamic execution truthfully.

### Allow `dynamic_execution_performed=true` for every profile

Rejected. This would weaken the static-only invariant and permit a static receipt to claim execution.

### Validate runtime fields independently and leave completeness to consumers

Rejected. `EvidenceBundle::validate()` is the runtime's own wire-integrity boundary. Accepting `requested_profile=linux_dynamic`, `dynamic_execution_performed=false`, and `disposition=completed` makes a semantically incomplete receipt structurally valid and pushes a security-relevant invariant onto every consumer.

### Treat `RuntimeBehavior` as independent descriptive evidence

Rejected. `RuntimeBehavior` is observed behavior. Allowing it when execution is false would make a receipt claim an observation that its own runtime manifest says did not occur.

### Repair only the Rust validator

Rejected. The JSON Schema is a consumer-visible compatibility surface. Leaving `const:false` or independent completeness/evidence fields in the schema would make Rust and wire validation disagree.

### Infer execution only from the presence of `RuntimeBehavior`

Rejected as insufficient. Evidence-kind presence does not replace an explicit runtime execution fact, and malformed or forged bundles still need cross-field validation. The required relation is consistency in both directions, not inference from one field alone.

### Make manifest/bundle validation profile-aware and encode the same wire rule

Selected after causal RED. Static-only receipts require no execution and no runtime-behavior evidence; unavailable dynamic receipts remain incomplete and contain no observed runtime behavior; completed dynamic receipts require actual execution. The schema encodes the same rule instead of validating each field independently.

## RED and causal evidence

Initial truthful-execution authority: `4cc901d7cb40bdc833e08b0c695ba12c27fa2f68`, `tests/artifact_analysis_dynamic_attestation_red.rs` (issue #52).

Rust cross-field hardening authority: `7fa9116a993d6854ad579db2512ff921edcb8611`.

Initial wire-schema hardening authority: `bb1001016f3efd3183fb903cdde242f0859279e5`.

Executable schema-semantics hardening authority: `e0ddf5abf96ff50718491ec4c59bb3728a71eed9`.

Observed-runtime consistency hardening authority: `2b540a0a2c24c7e53c183ff2a8ea1e85cd30daa8`.

Formatter-clean causal execution authority: `9d0da2a50e3247fd6cd2e52301d57408d404557d`, CI `35342943374`, review `5259590778`.

The executed regression covers these semantic boundaries:

1. otherwise-valid completed `LinuxDynamic` and `WindowsDynamic` bundles with `dynamic_execution_performed=true`, no network/credentials, and attributable `RuntimeBehavior` evidence must be representable;
2. unavailable Linux/Windows dynamic profiles with `dynamic_execution_performed=false` and `Inconclusive` remain valid receipts when they contain no observed runtime behavior;
3. the same unavailable dynamic state must not validate after changing only `disposition` to `Completed`;
4. `StaticOnly + dynamic_execution_performed=true` remains invalid;
5. `StaticOnly + dynamic_execution_performed=false + RuntimeBehavior` must be rejected;
6. unavailable dynamic `Inconclusive + dynamic_execution_performed=false + RuntimeBehavior` must be rejected;
7. the evidence-bundle JSON Schema must not globally force `dynamic_execution_performed=false` once approved dynamic completion is representable;
8. the schema must execute equivalent cross-field rules over representative serialized receipts, including evidence-kind consistency.

The causal run failed exactly the five previously unimplemented semantic cases while both guard controls passed. That is sufficient to proceed to the smallest receipt-contract GREEN; it does not establish worker/runtime containment or release readiness.

## Candidate causal GREEN

The current candidate changes only the Rust validation and checked-in JSON Schema needed by the executed RED.

Rust `RuntimeManifest::validate()` now rejects `dynamic_execution_performed=true` only for `StaticOnly`, while network and credential flags remain fail-closed. `EvidenceBundle::validate()` additionally requires `dynamic_execution_performed=true` when a Linux/Windows dynamic receipt is `Completed`, and rejects any `RuntimeBehavior` record when execution is false.

The Draft 2020-12 schema now exposes `dynamic_execution_performed` as a boolean and applies root-level `allOf` conditions that enforce the same three relationships: StaticOnly implies execution=false; a completed Linux/Windows dynamic receipt implies execution=true; execution=false excludes evidence containing `runtime_behavior`. Existing network and credential fields remain `const:false`.

No worker, sandbox launch path, network policy, credential policy, verdict authority, or analyzer result-ingestion behavior is changed by this repair. The candidate must still clear exact-head repository validation, rustfmt, the full locked workspace/all-target suite, Clippy/rustdoc with warnings denied, applicable production coverage, and the normal security/review gates before its semantics can be promoted.

## Release evidence

A dynamic artifact-analysis release requires more than a contract-valid boolean. The exact worker invocation must be bound to immutable artifact/profile/analyzer/runtime identity, capability-denying isolation, bounded CPU/RAM/PID/time/storage/output, bounded result ingestion, deterministic failure attribution, leak-free termination/cleanup, current-head security/coverage/SBOM/provenance, and protected integration. Static evidence must remain distinguishable from observed runtime behavior; an unavailable dynamic worker must never be upgraded from `Inconclusive` to `Completed` or retain fabricated `RuntimeBehavior` through serialization/consumer reconstruction; and Rust/schema validators must agree on those states.

## References

JSON Schema. (2022). *JSON Schema core: A media type for describing JSON documents, Draft 2020-12*. https://json-schema.org/draft/2020-12/json-schema-core

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification version 1.3.0*. https://specs.opencontainers.org/runtime-spec/?v=v1.3.0

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
