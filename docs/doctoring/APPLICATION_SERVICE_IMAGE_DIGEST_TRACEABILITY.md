# Application-Service Image Digest Traceability

## Problem

`ApplicationServiceRequest` accepts only registry image references pinned to a lower-case SHA-256 digest, and the Podman launch plan uses `--pull=never`. Those controls define immutable launch intent, but the application-service attestation path currently does not deserialize or verify the created container's applied `ImageDigest`. A lease can therefore attribute the requested immutable image to a container whose applied image identity was never bound back to that request.

This is an execution-integrity defect in the application-service infrastructure adapter. It is not an image-admission policy decision, and it does not transfer admission, verdict, or authorization authority into this repository.

## Authority and evidence levels

Podman container inspection exposes `.ImageDigest` as the applied image digest, distinct from `.Image` and `.ImageName`. The request digest is immutable consumer intent; `container inspect .ImageDigest` is backend-applied identity evidence. Neither value by itself proves the other, so the application-service runtime must compare them before loopback publication/readiness can become lease evidence.

`Image`, `ImageName`, local tags, generated resource names, mutable refs, or successful `--pull=never` creation are not substitutes for exact digest equality. Live process/isolation evidence remains separate from image identity, just as applied namespace/resource configuration remains separate from kernel enforcement.

## Current code path

- `src/application_service/mod.rs`: `ApplicationServiceRequest::validate()` requires a digest-pinned `image_reference`.
- `src/infrastructure/podman.rs`: `RootlessPodmanAdapter::plan_at()` uses `--pull=never` and passes the immutable image reference to `podman create`.
- `src/infrastructure/podman.rs`: application-service `ContainerInspection` does not currently deserialize `ImageDigest`.
- `src/infrastructure/podman.rs`: `verify_effective_isolation()` validates container identity, confinement, resource configuration and network object evidence, then queries the published port, but does not bind the applied image digest to `request.image_reference`.
- `ApplicationServiceLease` records the requested image reference, so publication without the applied-image comparison would overstate provenance.

## RED authority

Issue #46 owns the P0 image-binding gap. Test-bearing commit `3f7c05b01fb36653e1f13b5cb179881c6e61019f` adds `tests/podman_application_service_image_digest_red.rs` on Draft #19.

The fixture keeps the request digest immutable, returns one full long container ID, preserves otherwise-positive application-service confinement/resource inputs for the current lane, but reports a different `ImageDigest` from `podman container inspect`. Acceptance requires `IsolationVerificationFailed { control_name: "immutable_image_identity" }`, no port query/readiness publication, and runtime-owned cleanup attempts.

Production remains unchanged until that checked-in RED executes for the intended missing-image-binding cause. A queued, cancelled, predecessor, or source-inspection-only result is not causal RED evidence.

## Smallest causal GREEN after executed RED

1. Deserialize the applied `ImageDigest` from the exact created container inspection.
2. Extract the already-validated `sha256:<64-hex>` digest from `ApplicationServiceRequest.image_reference` without weakening request validation.
3. Require exact equality before loopback port publication/readiness/lease construction.
4. Reject missing, malformed, different-algorithm, or mismatched digest values fail closed.
5. Keep `.Image` and `.ImageName` diagnostic unless a separate versioned contract establishes stronger semantics.
6. Preserve #19 resource controls, #45 UTS/cgroup applied-state controls, #22/#23 network attachment proof, #20/#40/#42 lifecycle ownership, and positive LSM/live enforcement gates.

## Release evidence

A release candidate must additionally demonstrate on real rootless Podman that the exact service container's inspected `ImageDigest` equals the immutable request digest. The result must be tied to one unchanged protected integration SHA together with current coverage, review, SAST/security, positive effective-LSM, SBOM/provenance, reproducibility and rollback evidence.

## References

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
