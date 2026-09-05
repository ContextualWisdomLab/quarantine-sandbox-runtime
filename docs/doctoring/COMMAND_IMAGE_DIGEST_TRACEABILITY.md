# Command image-digest traceability

## Decision under review

A digest-pinned command request is launch intent. It becomes runtime evidence only after the created container is positively bound to the same immutable image digest.

`RootlessPodmanAdapter::run_command_at` currently supplies the validated `CommandExecutionRequest.image_reference` to `podman create` with `--pull=never`. `verify_command_isolation` then checks container identity, rootless/security/namespace/resource/network evidence, but the current `ContainerInspection` DTO does not deserialize Podman's applied `ImageDigest`. A different applied image can therefore pass the present attestation path if the remaining inspect and live-process evidence is positive.

Issue #37 owns this gap. Test-bearing commit `e23bfb0e6982169b437ae13d2f1cee29c2c59754` adds `tests/podman_command_execution_image_digest_red.rs`, which injects an otherwise-positive container inspection with a digest different from the immutable request and requires `IsolationVerificationFailed { control_name: "immutable_image_identity" }` before output is collected as trusted evidence.

## Evidence chain

- Request authority: `CommandExecutionRequest.image_reference`, already restricted to immutable SHA-256 registry-style identity.
- Launch intent: `RootlessPodmanAdapter::run_command_at` passes that exact reference with `--pull=never`.
- Backend-applied evidence: Podman `container inspect` exposes `.Image`, `.ImageDigest`, and `.ImageName`; `.ImageDigest` is the canonical `sha256:` image digest field.
- Missing production binding: `ContainerInspection` currently omits `.ImageDigest`, so `verify_command_isolation` cannot compare it with the request.
- Hostile RED: `tests/podman_command_execution_image_digest_red.rs` at `e23bfb0e6982169b437ae13d2f1cee29c2c59754`.
- Smallest causal GREEN after executed RED: deserialize `ImageDigest`, derive the already-validated requested digest without accepting mutable aliases, and require exact equality before command evidence can be trusted.
- Separate controls: #33/#36 own lifecycle/cleanup identity; #25 owns pre-payload effective attestation. Image digest equality must be incorporated into that pre-release proof rather than used as a substitute for seccomp/LSM/resource/mount evidence.

## Alternatives

Trusting `--pull=never` plus the requested digest without inspecting the created container is rejected because it proves CLI intent, not the applied container image. Treating `.ImageName` as authority is rejected because names are presentation/reference data and can vary independently of the immutable digest. Treating `.Image` as the requested digest is also rejected: Podman documents it as the local container image ID, while `.ImageDigest` is the digest field that directly corresponds to the request's `sha256:` identity.

## Risk and follow-up

Without this binding, a backend regression or incompatible Podman behavior can produce a false-GREEN command result attributed to an immutable request that was not the image actually instantiated. No release claim should advance until the focused RED executes for this cause, the minimal binding is GREEN, and a real rootless-Podman E2E demonstrates equality on the exact integrated head.

## References

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
