# Applied-Image Execution-Integrity Standard Traceability

## Decision scope

Issue #46 owns the P0 requirement that an application-service lease must not attribute the requested immutable image to a service container until the runtime has observed and verified the applied image identity on the exact acquired container.

This document narrows the evidence contract; it does not implement production GREEN and does not move image-admission, registry-policy, signing, or authorization ownership into the application-service bounded context.

## Current code evidence

Canonical application-service/network successor #127 exact `ba37ca70ca1dba08c87e2bdafebbda46cdf6fa22` does not deserialize an applied image digest in `ContainerInspection`. Its `verify_effective_isolation(...)` verifies the exact acquired container ID and then capability, seccomp, namespace, resource, LSM, network and port evidence without binding that container to the requested image digest. A present applied-image mismatch is therefore not observed on this path before readiness/lease publication.

Command/runtime #112 exact `19c450249e1779cd1d22c4837c8154fed91e1faf` has a narrower defect: it models `ImageDigest` as optional/defaulted and rejects a present mismatch, but missing/null evidence can skip `immutable_image_identity`. Focused missing-inspection owner #89 exact `8afa77c61af38121cbfcdc1051e6c45376e13a1b` remains a prerequisite for shared Podman inspection semantics.

Historical #19/#45/#46/#47 contain valid image/isolation witnesses that must be adopted ordinary/non-force onto dependency-safe current application-service ancestry. Historical network-cleanup command-shape assertions are not part of image evidence and must not be preserved as owner truth; #127/#141 own network cleanup and recovery syntax/authority.

## Evidence model and invariant

Requested immutable reference, backend-applied image identity, exact runtime-acquired container identity, and live isolation enforcement are separate evidence levels.

The application-service invariant is:

> Before port discovery, readiness, or lease publication, the runtime must observe a canonical applied digest on the exact acquired container and prove exact equality with the already-validated requested digest.

For this product contract the only admitted applied-digest representation is `sha256:<64 lower-case hexadecimal characters>`.

Although the OCI digest grammar supports multiple algorithms and encodings, QSR deliberately narrows applied-image authority to canonical SHA-256 because request validation already defines SHA-256 as the immutable product contract. A syntactically valid digest using another algorithm is therefore not equivalent authority and must not be normalized, transcoded, or accepted by semantic hash equivalence. Upper-case hexadecimal, wrong length, malformed text, missing/null evidence, and image name/tag/correlation values likewise cannot become authority.

## RED matrix before production GREEN

The current-ancestry witness must bind every case to the exact acquired container ID and distinguish these causes independently:

| Case | Inspection evidence | Expected image gate |
| --- | --- | --- |
| Positive control | exact canonical requested `sha256:<64 lower-case hex>` | pass to the next independent isolation gate |
| Canonical mismatch | different canonical SHA-256 digest | `immutable_image_identity` fail closed |
| Missing | `ImageDigest` absent | fail closed before port/readiness/lease |
| Null | `ImageDigest: null` | fail closed before port/readiness/lease |
| Wrong length | SHA-256 prefix with non-64-hex payload | fail closed |
| Upper-case/noncanonical | SHA-256 payload contains upper-case hex or otherwise noncanonical encoding | fail closed; do not normalize |
| Different algorithm | OCI-valid non-SHA-256 digest | fail closed; do not transcode |
| Name/correlation substitution | `.Image`, `.ImageName`, generated name or request text only | fail closed as insufficient applied-image evidence |

The witness must not prescribe `network rm --force`, generated-name deletion, or cleanup-selector syntax. It may require lifecycle cleanup to satisfy the canonical owner contract after failure, but network destructive authority remains #127/#141.

## Smallest causal GREEN after executed RED

1. Observe the applied image digest from the exact acquired container inspection.
2. Reject absent/null data at deserialization or the image-evidence ACL without substituting request data.
3. Validate the observed value against the product canonical form `sha256:<64 lower-case hexadecimal characters>`.
4. Extract the already-validated request digest without weakening request validation.
5. Require byte-for-byte equality before port discovery/readiness/lease publication.
6. Keep image identity independent from process/LSM/network/resource evidence and preserve all current lifecycle ownership constraints.

Real rootless-Podman acceptance must prove this equality on the exact service container. Synthetic fixtures remain unit/RED evidence only.

## Owner-safe sequence

1. Execute #89 current exact and integrate its valid shared-inspection delta ordinary/non-force through #14/#112.
2. Causally classify #127 current network/lifecycle exact; do not mix image GREEN into an unclassified owner head.
3. Adopt/adapt the complete valid #19/#45/#46/#47 image/isolation evidence onto current application-service ancestry without source copying or destructive restack.
4. Execute the full RED matrix above.
5. Apply only the minimum image-observation/canonicalization rejection/exact-comparison GREEN.
6. Reacquire one unchanged integrated exact head with repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc, owned-production statement/function/region/branch/edge/edge-case coverage, qualifying review/security, positive effective-LSM, and real rootless-Podman evidence.
7. Release remains separate: protected production integration, immutable tag/package/GitHub Release, SBOM, provenance, reproducibility and rollback are still required.

## Standards and primary-source traceability

Open Container Initiative. (2026). *OCI Image Format Specification: Content descriptors*. https://github.com/opencontainers/image-spec/blob/main/descriptor.md

The OCI descriptor specification defines a digest as a content identifier with `algorithm:encoded` syntax and states that compliant implementations should use SHA-256. QSR intentionally narrows the broader OCI grammar to its existing SHA-256 product contract rather than treating all OCI-valid algorithms as interchangeable authority.

OpenContainers Authors. (2026). *go-digest: algorithm.go*. https://github.com/opencontainers/go-digest/blob/master/algorithm.go

`go-digest` defines SHA-256 as the canonical algorithm and uses lower-case hexadecimal encoding only. This supports the exact QSR applied-evidence representation `sha256:<64 lower-case hexadecimal characters>` and the decision to reject rather than normalize noncanonical text.

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman container inspection is the backend observation surface. QSR must bind the observed applied identity to the exact runtime-acquired container ID rather than infer it from the request, an image name, a tag, or a generated runtime correlation.

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

NIST SP 800-190 treats discrete image identification, trusted-image enforcement, and validation before execution as part of the container trust chain. QSR's applied-image binding is the runtime evidence layer that prevents a lease from overstating which immutable image actually backs the service container.
