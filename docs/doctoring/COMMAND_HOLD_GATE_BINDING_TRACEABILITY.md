# Command hold-gate binding traceability

Status: active Draft evidence for issue #25 and ADR-0008. This document does not authorize release.

## Problem

`RootlessPodmanAdapter::run_command_at` still serializes the requested consumer argv into Podman's OCI entrypoint during `create`. `podman start` can therefore make hostile consumer code runnable before the runtime has positively sampled effective seccomp, capability and LSM state. Cleanup after a failed attestation does not undo code execution.

The package already contains the runtime-owned `qsr_runtime_gate`, and the dedicated real-rootless capability test has shown that the gate can remain the sole running process, expose effective process evidence and `exec()` exact consumer argv after explicit release. The missing production boundary is now decomposed so gate delivery/initial-process binding is proven separately from release-channel design.

## Causal evidence

- Original hostile payload-side-effect RED: exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, verify `102428599037` and branch coverage `102428599228`. The payload became runnable at `start` before live attestation failed.
- Packaged gate capability: minimum production `3bd5d4319aba7c23982bbd18c77d424c25c3a510` introduced `src/bin/qsr_runtime_gate.rs`; exact CI `34381498621` made the focused packaging regression GREEN before the unchanged pre-attestation RED stopped the broader run.
- Real held-gate capability: the dedicated rootless-Podman test compiles the production gate source as a static `x86_64-unknown-linux-musl` executable, mounts it read-only at `/qsr-runtime-gate`, verifies it is the only process before release, samples seccomp/capability/ambient/label evidence, then releases exact consumer argv.
- Gate-binding RED: test-only `98b75125a619c981086ec1babb07fbc298ca7601` added `tests/podman_command_execution_hold_gate_binding_red.rs`. Native CI `34393277076` stopped at rustfmt before semantic execution; formatter-only `5a591a582ca955ccd24b8666a779de279062fae7` changed no assertion or production behavior.
- Exact `5a591a582ca955ccd24b8666a779de279062fae7`, native CI `34393466321`, verify `102607270660`, then passed repository policy, dependency lock, coverage-parser checks, rustfmt and all preceding command-runtime tests through the exact ENTRYPOINT regression. It failed `podman_create_binds_runtime_gate_as_initial_process_before_consumer_argv` for the intended cause: the observed `create` invocation still contained `--entrypoint=["payload-sentinel","argument with spaces"]` and no runtime-owned `/qsr-runtime-gate` delivery/binding.

No predecessor GREEN transfers to a moved head. This evidence proves the missing binding, not a completed hold/attest/release adapter.

## Constraints and invariants

The repair must keep these existing contracts intact:

- exact requested argv semantics, including whitespace and argument boundaries;
- immutable acquired container ID as destructive lifecycle authority;
- digest-pinned/no-pull hostile images;
- rootless execution, read-only rootfs, no-new-privileges, capability drop, private namespaces, deny-by-default network, resource limits and bounded `/tmp`;
- read-only/noexec/nosuid/nodev source staging when present;
- pre-start configured-state contradiction checks without relabeling them effective evidence;
- effective seccomp, capability and LSM evidence sampled while hostile consumer argv is still non-runnable;
- bounded output, workload timeout semantics and cleanup-error precedence;
- fail closed on gate absence, identity mismatch, evidence failure, release-channel failure or cleanup uncertainty.

The gate must be runtime-owned and immutably identified. A binary supplied by the hostile image cannot be an attestation authority. Architecture compatibility is part of the delivery contract; the current real E2E's explicit static musl build is capability evidence, not yet a general distribution mechanism.

## Alternatives considered

### Keep consumer argv as OCI entrypoint and attest after `start`

Rejected. This is the exact P0 demonstrated by the original payload-side-effect RED and the gate-binding RED.

### Move only `container inspect` or `podman init` before `start`

Retained as an earlier contradiction check, rejected as effective process proof. Configured state cannot establish the final running process's seccomp/LSM/capability state.

### Start an inert hostile-image command and later use plain `podman exec`

Rejected as the canonical boundary. It moves trust to image content and does not by itself bind the consumer transition to the attested runtime-owned process identity.

### Runtime-owned hold gate

Selected as the current Proposed ADR-0008 direction. The gate is the initial OCI process, remains held while effective isolation is sampled, and replaces itself with exact consumer argv only after a bounded positive release decision. Production acceptance still requires immutable architecture-compatible delivery/digest binding and a bounded one-time release channel.

## Minimum next source slice

The next production change must first make Podman `create` deliver the runtime-owned gate read-only and bind `/qsr-runtime-gate` as the initial OCI process while preserving exact consumer argv behind it. The same implementation must then prove gate identity and effective isolation before release. If a safe immutable gate artifact cannot be resolved for the current host/image architecture, command execution must fail closed before `start` rather than fall back to the consumer entrypoint.

After that source slice, rerun the new gate-binding RED and the original payload-side-effect RED on the same exact head. A GREEN binding test without bounded release, effective attestation or real positive-LSM evidence is not release authority.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Open Container Initiative. (2026). *Open Container Initiative runtime specification: POSIX-platform hooks*. https://github.com/opencontainers/runtime-spec/blob/main/config.md

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Podman Authors. (2026). *podman — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
