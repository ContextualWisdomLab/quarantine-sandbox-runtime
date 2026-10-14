# Podman Missing Isolation Evidence Traceability

Issue #73 owns a P0 infrastructure-ACL defect in the application-service Podman adapter. This record distinguishes an explicitly observed secure value from absence of the backend field that would be needed to support that value.

## Current authority

The dependency root for this RED is Draft #1 exact `17e79aab32aa75efd26bcccf92415474af9bc69b`. Its checkout-credential repair is already proven on predecessor `7482108c0b74f58f447722a98330f9ad44215eec` / native CI `34089522598`; this child does not reopen that workflow repair or issue #83's production-coverage cause.

At the parent source, `ContainerInspection.EffectiveCaps` and `ContainerInspection.BoundingCaps` use `#[serde(default)]`, and `NetworkInspection.dns_enabled` also uses `#[serde(default)]`. The application-service verifier interprets empty capability lists and `dns_enabled=false` as secure evidence. Serde therefore makes two materially different backend observations indistinguishable at this ACL: an explicit empty/false value and a missing field filled with `Default::default()`.

Test authority begins at `tests/podman_missing_isolation_evidence_red.rs`. It removes exactly one required field at a time from an otherwise-positive fake-Podman inspection and requires `MalformedIsolationInspection` for the exact inspection operation. A positive control keeps explicit empty capability arrays and explicit `dns_enabled=false`; those values must pass parsing/isolation admission and continue to the independent readiness gate.

Fake-Podman coverage is deterministic contract evidence only. It does not prove actual container confinement or replace the dedicated rootless/positive-LSM release lane.

## Problem and security consequence

A missing backend security field is unavailable evidence, not affirmative evidence that the corresponding control is secure. Defaulting missing capability arrays to empty can falsely attest that no capabilities are present. Defaulting missing DNS state to `false` can falsely attest DNS denial. Truncated, version-skewed, or otherwise incompatible Podman inspection JSON can therefore cross the infrastructure ACL with a stronger security meaning than the backend actually supplied.

This violates the repository's fail-closed P0 contract. `infrastructure::podman` owns translation of Podman inspection data into backend-neutral runtime evidence; Supporting contexts must receive positively observed facts rather than inferred secure defaults.

## Decision

After the focused exact-head RED executes for the intended cause, the smallest causal GREEN is to make `EffectiveCaps`, `BoundingCaps`, and `dns_enabled` required deserialization fields by removing only the corresponding `#[serde(default)]` attributes. Existing `parse_json` handling can then classify a missing required field as `ApplicationServiceError::MalformedIsolationInspection` for `container_inspect` or `network_inspect`.

Explicit `EffectiveCaps: []`, `BoundingCaps: []`, and `dns_enabled: false` remain valid evidence when all other controls are valid. The repair must not coerce missing fields to a secure value, add fallback inference from unrelated fields, weaken process-level capability/LSM/seccomp checks, or promote fake-backend tests to release-grade confinement proof.

Rejected alternatives:

- keeping `#[serde(default)]` and trying to infer presence later, because presence information is already destroyed during deserialization;
- changing the fields to `Option` and treating `None` as secure, because that preserves the same false-GREEN semantics;
- accepting absence for backend compatibility, because compatibility uncertainty at a P0 security evidence boundary must fail closed;
- substituting process-top capability evidence for missing container-inspect capability fields, because the two observations are independent defense-in-depth evidence and one must not silently manufacture the other.

## DDD and test mapping

- `src/infrastructure/podman.rs`: Podman JSON translation and malformed-inspection classification.
- `tests/podman_missing_isolation_evidence_red.rs`: missing-field RED and explicit-value positive control.
- `application_service`: consumes only the stable failure vocabulary; no Podman DTO ownership moves into the Supporting context.
- `sandbox_execution`: remains owner of backend-neutral isolation truth and does not gain Podman-specific field semantics.

Issue #73 remains separate from applied UTS/cgroup namespace evidence (#45), image-digest binding (#46), exact network attachment (#22/#23), resource enforcement (#7/#19), lifecycle ownership (#40/#42), and the root production-coverage repair (#83).

## Evidence and release implications

A GREEN parser/ACL repair establishes only that required Podman inspection evidence cannot disappear silently. Release authority still requires one unchanged integrated candidate to satisfy formatting, locked tests, Clippy, rustdoc, repository validation, exact 100% owned production statement/function/region/branch coverage, review/security gates, real rootless runtime E2E, positive selected-LSM evidence, protected `develop` integration, SBOM/provenance/reproducibility/rollback, and immutable publication.

## References

Podman Authors. (n.d.). *podman-container-inspect — Display a container's configuration*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman Authors. (n.d.). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Serde Project. (n.d.). *Default value for a field*. Serde documentation. https://serde.rs/attr-default.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
