# Application-service direct-argv / image ENTRYPOINT traceability

Reviewed 2026-09-06 KST against Draft PR #19 and issue #47. Latest focused test-bearing authority is `115945244d3e7fd2e6a2b04090c16935233a011e`. Production behavior is intentionally unchanged until that RED executes for the intended entrypoint-inheritance cause.

## Problem

`ApplicationServiceRequest.command` is documented as optional direct argv, and the PRD requires process-boundary coverage for direct argv invocation without a shell. The Podman launch plan currently places the immutable image operand before `request.command`, producing the CLI shape `podman create [options] IMAGE COMMAND [ARG...]`.

That shape does not by itself make `COMMAND` the executable. An OCI image may define an `Entrypoint`; Podman preserves that image entrypoint unless container creation explicitly overrides it, and values supplied after `IMAGE` become command/options for the entrypoint. A consumer-visible lease can therefore describe a service whose executable identity differs from the direct argv the request appears to authorize.

## Authority chain

1. `src/application_service/mod.rs` defines `ApplicationServiceRequest.command` as direct argv and rejects empty, oversized or control-bearing entries.
2. `docs/PRD.md` requires the P0 application service to invoke the application without a shell and requires process-boundary direct-argv tests.
3. `src/infrastructure/podman.rs::RootlessPodmanAdapter::plan_at` currently appends the validated command after the image and emits no `--entrypoint` override.
4. Podman `podman-create(1)` documents that `--entrypoint` replaces the image ENTRYPOINT and that a configured image ENTRYPOINT remains the executable while COMMAND supplies additional options/arguments.
5. The OCI Image Specification defines `Entrypoint` and `Cmd` as image defaults that may be replaced at container creation.
6. Issue #47 owns the resulting application-service contract/execution-integrity gap. `tests/podman_application_service_entrypoint_red.rs` requires a non-empty direct argv to be represented as one exact JSON-array entrypoint and forbids a duplicate post-image command layer.

## Evidence levels

The first GREEN may prove only **launch-plan intent**: exact validated argv boundaries are encoded without a shell and image ENTRYPOINT inheritance is disabled for non-empty `command`.

That is not yet proof of the effective process. Release evidence must additionally bind the exact created service container to the requested immutable image (#46), retain exact lifecycle ownership (#40/#42), and use a reliable Podman/OCI inspection or real rootless execution observation to demonstrate that the effective executable/argv matches the contract. Positive LSM, namespace, resource, network, readiness and cleanup evidence remain independent gates.

## Alternatives considered

- **Keep appending COMMAND after IMAGE.** Rejected for the current direct-argv wording because an image ENTRYPOINT can remain the executable.
- **Join argv into a shell string.** Rejected because it destroys argument boundaries and violates the no-shell contract.
- **Require all admitted images to have no ENTRYPOINT.** Rejected as an implicit image-authoring convention rather than a runtime-enforced contract; it would also require separate image-config attestation before launch.
- **Treat `command` as arguments to the image ENTRYPOINT.** Viable only as an explicit, versioned product-contract change across Rust API/schema/PRD/consumer documentation. It is not compatible with the current direct-argv wording by assumption alone.
- **Override ENTRYPOINT with the complete argv for non-empty commands.** Selected causal direction after executed RED because Podman directly supports JSON-array entrypoints and preserves argument boundaries. Empty `command` continues to mean image-defined defaults unless a later versioned contract changes that behavior.

## Risk and follow-up

Until #47 is GREEN, a digest-pinned and otherwise well-isolated service can still execute an image-defined binary in front of the caller-provided argv. The risk is contract/evidence misattribution rather than proof of sandbox escape, so #47 is P1. Do not promote it to release evidence from source inspection or a queued run.

After the RED executes for its intended cause, implement the smallest plan repair, add effective process-argv evidence on the supported real rootless Podman version, keep documentation/schema wording code-current, and reacquire exact-head CI, coverage, security, review, positive-LSM and protected-integration evidence.

## References

Open Container Initiative. (2025). *Open Container Initiative Image Specification — Image configuration*. https://specs.opencontainers.org/image-spec/config/

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html
