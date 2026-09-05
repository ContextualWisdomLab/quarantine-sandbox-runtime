# Application-service create-identifier authority traceability

Status: RED-only evidence on Draft #21. Production behavior is intentionally unchanged until the checked-in RED executes for the intended cause.

## Problem and security boundary

`RootlessPodmanAdapter::launch_at` parses successful `podman create` stdout with `parse_backend_identifier`. The current parser accepts any non-empty UTF-8 token up to 128 characters as long as it contains no whitespace or control characters. Issue #40 requires the create result to become the immutable lifecycle/destructive authority after creation, so that parser contract is too weak: a Podman name, short ID, arbitrary token, or non-hex 64-character value must not be promoted into ownership authority.

The generated `qsr-app-*` name remains runtime-owned correlation metadata and is still the safe cleanup selector when create output itself is malformed, because a failed/malformed create response may nevertheless have persisted the requested named resource. The untrusted create-output value must not be used for `start`, `container inspect`, `top`, `port`, `stop`, or `rm` until it has been proven to be one full long container ID.

## Evidence chain

1. `podman create` creates but does not start the container and prints the container ID to stdout.
2. Podman lifecycle commands accept IDs or names, so accepting a human-friendly or abbreviated selector is semantically different from retaining the full immutable ID returned by create.
3. NIST SP 800-190 assigns the runtime the role of establishing and maintaining container isolation and lifecycle controls; NISTIR 8176 treats Linux container security as an assurance problem, not only a configuration-intent problem.
4. Therefore the runtime must validate the identifier before it becomes lifecycle/destructive authority, then bind later operations to that exact long ID.

## RED

Test-bearing commit `52387cf69c3b32356caba8bc126e659c6d94461d` adds `tests/podman_application_service_create_identifier_red.rs` with three independent hostile create outputs:

- `foreign-container` — name-like selector;
- `0123456789ab` — short hexadecimal ID;
- 64 `g` characters — full-width but non-hex token.

Each case preserves otherwise-valid backend and network creation. Acceptance requires `ApplicationServiceError::MalformedIsolationInspection { operation: "container_create" }` before any `start`, `container inspect`, `top`, or `port` call can select a post-create target. Runtime-owned cleanup may still remove the requested generated name and network.

The existing #40 name-rebinding RED remains complementary: it proves that a valid acquired long ID must continue to be used after create rather than resolving the mutable name again.

## Smallest causal GREEN after executed RED

`parse_backend_identifier` should accept exactly one 64-character hexadecimal container ID and reject every other representation before lifecycle use. The later #40 production repair should retain that validated long ID in runtime-owned provenance and use it for every supported post-create container operation. Keep `inspect.Id == acquired_id` as defense in depth.

Do not treat syntax alone as cleanup authorization for a deserialized lease; issue #42 separately requires runtime-owned cleanup provenance. Do not replace the public `sandbox_id` correlation contract without a versioned contract change.

## Rejected alternatives

- **Accept short IDs.** Rejected because abbreviation reintroduces selector ambiguity and is unnecessary when create already returns the full ID.
- **Accept names when they match `qsr-app-*`.** Rejected because a name is a re-resolvable selector, not immutable ownership evidence.
- **Accept arbitrary 64-character strings.** Rejected because width alone does not establish Podman's hexadecimal container-ID grammar.
- **Use inspect-by-name and compare `Id` afterward.** Rejected as the primary boundary because a foreign resource has already been selected before the comparison.
- **Make `ApplicationServiceLease.sandbox_id` the acquired ID.** Rejected as an incidental fix because it silently changes the public correlation contract and does not solve #42 cleanup authorization.

## References

Podman. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (2026). *podman-start — Start one or more containers*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Chandramouli, R. (2017). *Security assurance requirements for Linux application container deployments* (NIST Interagency Report 8176). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.IR.8176
