# Command image-digest traceability

## Decision under review

A digest-pinned command request is launch intent. It becomes runtime evidence only after the created container is positively bound to the same immutable image digest.

`RootlessPodmanAdapter::run_command_at` supplies the validated `CommandExecutionRequest.image_reference` to `podman create` with `--pull=never`. Issue #37 requires the command-runtime attestation path to bind Podman's applied `ImageDigest` back to that immutable request before command output can become trusted evidence.

## Causal RED and current repair boundary

The hostile mismatch regression is `tests/podman_command_execution_image_digest_red.rs`, introduced by test-bearing commit `e23bfb0e6982169b437ae13d2f1cee29c2c59754`.

On canonical PR #14 exact predecessor `4a59aaef4876b80e58782067f7737b110a3ebe77`, native CI `34251341897` reached verify job `102146265381` after repository validation, coverage-parser tests and `cargo fmt --check` had passed. The earlier exact-command ENTRYPOINT regression and the >=128-bit runtime-identity regression also passed on that same execution. The suite then failed exactly at `podman_command_execution_image_digest_red::mismatched_applied_image_digest_fails_closed_before_command_evidence_is_trusted`: the request pinned `sha256:eeee…`, the created container reported `ImageDigest: sha256:ffff…`, yet the runtime returned `Ok(CommandExecutionResult)` and collected the fixture output `must-not-be-trusted`.

Commit `8f3f7f639c9bb04a6e49593663fe5fef787794f5` is the smallest mismatch-binding candidate on the same owner branch. `ContainerInspection` now deserializes Podman's `ImageDigest` as optional backend evidence. When Podman supplies the field, `verify_command_isolation` compares it exactly with the digest portion of the already-validated immutable request and fails as `IsolationVerificationFailed { control_name: "immutable_image_identity" }` before wait/log evidence is trusted if they differ.

This candidate is deliberately narrower than the full issue acceptance. Missing `ImageDigest` remains tolerated so existing command fixtures and the application-service sibling are not silently rewritten as proof. Issue #37 therefore stays open: missing-field and malformed-field fail-closed behavior, fixture migration, and real rootless-Podman equality evidence still require their own executable edge evidence before this control is complete. Application-service image binding remains separately owned by #46.

## Evidence chain

- Request authority: `CommandExecutionRequest.image_reference`, restricted to immutable lower-case SHA-256 registry-style identity.
- Launch intent: `RootlessPodmanAdapter::run_command_at` passes that exact reference with `--pull=never`.
- Backend-applied evidence: Podman `container inspect` exposes `.Image`, `.ImageDigest`, and `.ImageName`; `.ImageDigest` is the canonical `sha256:` image digest field.
- Executed mismatch defect: exact predecessor `4a59aaef4876b80e58782067f7737b110a3ebe77`, CI `34251341897`, verify `102146265381` trusted output despite a different applied digest.
- Minimum mismatch repair: `8f3f7f639c9bb04a6e49593663fe5fef787794f5` deserializes `ImageDigest` and rejects a supplied non-equal digest before command evidence is trusted.
- Remaining fail-closed gap: absence of `ImageDigest` is not yet rejected and is not release-grade attestation.
- Separate controls: #33/#36 own lifecycle/cleanup identity; #25 owns pre-payload effective attestation. Image digest equality must be incorporated into that pre-release proof rather than used as a substitute for seccomp/LSM/resource/mount evidence.

## Alternatives

Trusting `--pull=never` plus the requested digest without inspecting the created container is rejected because it proves CLI intent, not the applied container image. Treating `.ImageName` as authority is rejected because names are presentation/reference data and can vary independently of the immutable digest. Treating `.Image` as the requested digest is also rejected: Podman documents it as the local container image ID, while `.ImageDigest` is the digest field that directly corresponds to the request's `sha256:` identity.

Silently accepting a present but mismatched `ImageDigest` is rejected by the current candidate. Conversely, treating the current optional DTO field as proof that missing evidence is safe is also rejected: optional deserialization is only a compatibility step while the remaining fail-closed fixture migration is still open.

## Risk and follow-up

Without complete binding, a backend regression or incompatible Podman behavior can produce a false-GREEN command result attributed to an immutable request that was not positively proven to be the image actually instantiated. No release claim should advance until the mismatch candidate passes fresh exact-head CI, missing/malformed digest evidence fails closed, and a real rootless-Podman E2E demonstrates equality on the exact integrated head.

## References

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
