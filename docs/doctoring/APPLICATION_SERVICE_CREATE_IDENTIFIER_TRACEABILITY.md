# Application-service create-identifier authority traceability

Status: causal RED reproduced on current-root ancestry; minimum parser repair is under exact-head verification on Draft #21.

## Problem and security boundary

`RootlessPodmanAdapter::launch_at` parses successful `podman create` stdout with `parse_backend_identifier`. Before the current repair, the parser accepted any non-empty UTF-8 token up to 128 characters as long as it contained no whitespace or control characters. Issue #40 requires the create result to become the immutable lifecycle/destructive authority after creation, so that parser contract is too weak: a Podman name, short ID, arbitrary token, or non-hex 64-character value must not be promoted into ownership authority.

The generated `qsr-app-*` name remains runtime-owned correlation metadata and is still the safe cleanup selector when create output itself is malformed, because a failed/malformed create response may nevertheless have persisted the requested named resource. The untrusted create-output value must not be used for `start`, `container inspect`, `top`, `port`, `stop`, or `rm` until it has been proven to be one full long container ID.

## Evidence chain

1. `podman create` creates but does not start the container and prints the container ID to stdout.
2. Podman lifecycle commands accept IDs or names, so accepting a human-friendly or abbreviated selector is semantically different from retaining the full immutable ID returned by create.
3. NIST SP 800-190 assigns the runtime the role of establishing and maintaining container isolation and lifecycle controls; NISTIR 8176 treats Linux container security as an assurance problem, not only a configuration-intent problem.
4. Therefore the runtime must validate the identifier before it becomes lifecycle/destructive authority, then bind later operations to that exact long ID.

## RED

The RED originated in test-bearing commit `52387cf69c3b32356caba8bc126e659c6d94461d` and was dependency-safely restacked onto current root `5c6a44bb2b35eb17d0315d72db242f4488c3c426`. After rustfmt-only prerequisite repairs, exact head `a22ddda022dcdb5b03378c368fce71ad86dc7fa3` ran native CI `34298464566`. Exact checkout, dependency lock, repository policy, coverage-parser tests, rustfmt, and the existing application-service suite passed before `tests/podman_application_service_create_identifier_red.rs` executed.

All three hostile create outputs then failed for the intended causal reason:

- `foreign-container` — name-like selector;
- `0123456789ab` — short hexadecimal ID;
- 64 `g` characters — full-width but non-hex token.

Each expected `ApplicationServiceError::MalformedIsolationInspection { operation: "container_create" }`, but the implementation advanced to `container_start` and returned `BackendCommandFailed { operation: "container_start" }`. This proves the create-output admission boundary, rather than a formatting/repository prerequisite, was missing. The fixture also proved no post-create lifecycle operation should be reachable for those values.

The existing #40 name-rebinding RED remains complementary: it proves that a valid acquired long ID must continue to be used after create rather than resolving the mutable name again.

## Smallest causal GREEN

Candidate commit `5dc13da83ceda911464ac0d3264495d6f3a747df` changes only `parse_backend_identifier`: the admitted value must be exactly 64 ASCII hexadecimal characters. Fresh compare from the causal RED shows one production file changed, with two added and nine removed lines; no retry, normalization, name fallback, short-ID acceptance, or lifecycle semantic change was introduced.

This is deliberately narrower than the later #40 lifecycle-ownership repair. Once a valid acquired long ID is admitted, runtime-owned provenance must still use it for every supported post-create container operation. Keep `inspect.Id == acquired_id` as defense in depth.

Exact native CI `34298773775` belongs to candidate `5dc13da8...` and must complete before this parser repair is called GREEN. Predecessor checks do not transfer.

Do not treat syntax alone as cleanup authorization for a deserialized lease; issue #42 separately requires runtime-owned cleanup provenance. Do not replace the public `sandbox_id` correlation contract without a versioned contract change.

## Rejected alternatives

- **Accept short IDs.** Rejected because abbreviation reintroduces selector ambiguity and is unnecessary when create already returns the full ID.
- **Accept names when they match `qsr-app-*`.** Rejected because a name is a re-resolvable selector, not immutable ownership evidence.
- **Accept arbitrary 64-character strings.** Rejected because width alone does not establish Podman's hexadecimal container-ID grammar.
- **Use inspect-by-name and compare `Id` afterward.** Rejected as the primary boundary because a foreign resource has already been selected before the comparison.
- **Retry or normalize malformed output.** Rejected because it would hide the evidence-integrity failure or widen an immutable-identity boundary.
- **Make `ApplicationServiceLease.sandbox_id` the acquired ID.** Rejected as an incidental fix because it silently changes the public correlation contract and does not solve #42 cleanup authorization.

## References

Podman. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (2026). *podman-start — Start one or more containers*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Chandramouli, R. (2017). *Security assurance requirements for Linux application container deployments* (NIST Interagency Report 8176). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.IR.8176
