# Command hold/attest/release traceability

Status: **Proposed / P0 open / backend capability and packaged-gate slice proven**

This note records the current evidence and next acceptance boundary for issue #25. It does not mark ADR-0008 Accepted and it does not authorize a release.

## Causal P0 evidence

Canonical command-runtime PR #14 repeatedly executes `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation`. Exact `7da7d3b36ec8830ab16d776e5ed1a93f7a16b8ad`, native CI `34379177369`, verify `102559489320` passed repository policy, coverage-parser tests, rustfmt, the primary 15 command-execution tests, create-failure ownership, exact ENTRYPOINT argv, invocation identity, image-digest binding, init-hold capability, live-attestation fail-closed behavior, bounded log storage, malformed-create cleanup, exact mount-set, namespace configuration, strict output encoding, Podman 4.9 compatibility, and acquired-ID lifecycle ownership. It then failed exactly at the P0 regression because production still makes hostile consumer argv the OCI program released by `podman start` before live effective-process attestation.

The current production order remains:

`create(consumer argv) -> acquire exact ID -> podman init -> configured-state verification -> podman start -> live process verification`

`podman init` is a valid earlier hold point, but it does not make the future consumer process's effective seccomp/LSM/capability state observable. Cleanup after a failed post-start attestation cannot undo hostile code execution.

## Real rootless backend capability proof

The repository has an independent real-runtime proof that a runtime-owned hold/attest/release mechanism is feasible on the supported hosted negative environment. Exact `7da7d3b36ec8830ab16d776e5ed1a93f7a16b8ad`, native CI `34379177369`, hosted job `102559489017` completed GREEN on Ubuntu 24.04 with rootless Podman 4.9.3.

That predecessor E2E compiled a checked-in static `x86_64-unknown-linux-musl` gate fixture, marked it read/execute-only, hashed it, mounted it read-only at `/qsr-runtime-gate`, and selected it as the container ENTRYPOINT instead of the hostile consumer. The gate blocked on a runtime release token and called `exec()` on the exact preserved consumer argv only after release.

The real E2E proved, on one exact head:

- the runtime-owned gate becomes the only running container process while the consumer sentinel is absent;
- the held process exposes effective seccomp mode and empty effective/bounding/inheritable/permitted capability sets;
- Podman 4.9.3 exposes host PID through `hpid`, allowing ambient capability evidence to be checked from `/proc/<hpid>/status` as `CapAmb=0` without inventing an unsupported `capamb` top descriptor;
- a process security label is observable before release, while the hosted lane correctly does not promote an unavailable effective LSM to positive confinement;
- the mounted gate SHA-256 is unchanged while held and through the exec transition;
- explicit release causes the gate to replace itself with the exact consumer argv, producing the expected sentinel only after release;
- exact-ID cleanup and the repository leak scan succeed;
- the same hosted lane still proves unavailable positive LSM fails closed rather than being treated as confinement.

This closes the backend primitive feasibility question only. It does not close issue #25 because production `RootlessPodmanAdapter::run_command_at` still starts the consumer directly.

During capability-test RCA, Podman 4.9.3 also exposed two representation constraints that are retained rather than hidden by retries or weakened assertions. `podman top` supports `capeff`, `capbnd`, `capinh`, `capprm`, `hpid`, `label`, and `seccomp`, but not `capamb`; an unsupported descriptor can fall through to `ps` behavior. The test therefore queries supported descriptors separately and obtains ambient capability evidence through the kernel process status. Podman may render an empty capability set as `none` rather than an all-zero hexadecimal string, so the regression applies the same semantic empty-set normalization already used by production while rejecting every non-empty set.

## Packaged production gate: RED -> focused GREEN

The real capability fixture left a product boundary unresolved: the package itself had no runtime-owned gate executable. Test-only exact `304d63c2722a35b9f05f8466bbea81492a1533a6`, native CI `34381329199`, verify `102566637062`, passed exact checkout, dependency lock, repository policy, coverage-parser tests and rustfmt and then failed `command_hold_gate_packaging_red::unix::package_exposes_fail_closed_runtime_gate_with_exact_consumer_argv` for the intended reason: Cargo exposed no `CARGO_BIN_EXE_qsr_runtime_gate`.

Minimum production exact `3bd5d4319aba7c23982bbd18c77d424c25c3a510` added `src/bin/qsr_runtime_gate.rs`. The binary accepts one non-empty release token bounded to 128 bytes and the exact consumer program/argv, emits and flushes `QSR_GATE_READY`, reads at most one bounded token from stdin, rejects invalid invocation/channel/token fail closed, and uses Unix `exec()` on exact authorization so shell joining cannot alter consumer argv. Public failure behavior is bounded; it does not expose raw errno or host paths.

Native CI `34381498621` executed that exact production head. Verify `102567695133` and statement-coverage job `102567694962` both compiled the binary and passed the packaging regression, then continued until the original issue #25 payload-side-effect regression failed because `RootlessPodmanAdapter::run_command_at` still starts the consumer directly. Hosted negative rootless/AppArmor job `102567695588` was GREEN. This is exact-head focused GREEN for packaging and exact-head causal RED for production hold/attest/release, not an integrated GREEN.

The previous E2E still exercised a duplicate test fixture rather than the production gate. Follow-up exact `a65381d50013f42c91b796e71755048ab3b7c584` changed `tests/podman_hold_gate_capability.rs` to compile `src/bin/qsr_runtime_gate.rs` directly as `x86_64-unknown-linux-musl`, and `013aeade4ae3a7fa86484c82b4bc9336c0e7e4ad` removed the now-redundant `tests/fixtures/qsr_hold_gate.rs`. Current CI `34382511674` is pending at this documentation cut, so the production-gate real-Podman reuse is not yet claimed GREEN. Static/versioned release artifact generation, architecture selection, exact digest delivery, and adapter integration remain open.

## Primary authority

OCI defines `created` as a state in which the container process has not executed the user-specified program. During `create`, all configuration except `process` must be applied, `process.args` must not be applied until `start`, and remaining process properties may be applied during `create`. `start` runs the user-specified program. Portable code therefore cannot assume every process-specific security property is final merely because the container has been initialized or created.

Podman documents `podman init` as preparing a container for start without starting it. Podman `top` provides process descriptors including seccomp, capability sets, host PID and security label; its supported descriptor surface is version-sensitive and must be treated as a backend ACL rather than guessed.

Linux preserves seccomp filters across `execve(2)` when `execve` is permitted. `no_new_privs` cannot be unset and is preserved across `execve(2)`. The capability bounding set constrains capabilities that can be gained during `execve(2)`. The real rootless E2E demonstrates that these properties can be sampled while a runtime-owned gate is held before the exact consumer exec transition.

## Selected production boundary

The selected architecture remains a **runtime-owned execution gate** as the OCI user process:

1. Bind the exact acquired container ID to an immutable architecture-compatible runtime gate instead of hostile consumer argv.
2. Start the gate under the final container process controls while it remains blocked before executing consumer code.
3. Observe effective seccomp, capability and LSM evidence for that exact running gate through the backend's live process evidence path.
4. Evaluate the selected isolation policy against bounded evidence.
5. Release only after positive evaluation; the gate replaces itself with the exact consumer argv via `execve`.
6. Treat gate delivery, identity, evidence, release-channel or exec failure as fail-closed and perform exact-ID cleanup.

The real E2E changes this from an unproven backend hypothesis to a proven capability candidate, and the packaged binary closes the source/package-exposure prerequisite. Production acceptance still requires a versioned static delivery/identity contract and a deterministic integration RED showing that `run_command_at` itself creates the trusted gate as PID 1, evaluates effective evidence while held, and cannot release consumer argv on any negative path.

## Rejected shortcuts

- Moving static `container inspect` earlier is insufficient because configured state is not the same as effective process state.
- `podman init` plus static evidence is only a pre-release contradiction check; it is not full effective attestation.
- Starting the hostile consumer and failing after `podman top` is too late; cleanup cannot undo execution.
- Plain `podman exec` after an inert hostile-image entrypoint does not establish a trusted gate and creates another executable authority surface.
- A shell/script or helper supplied by the hostile image cannot be the attestation authority.
- A `startContainer` hook alone is not accepted as proof of the later consumer process's exact effective state.
- Retry, sleep, mutex serialization or weakened assertions are not fixes for the release-order defect.
- Treating Podman display spelling such as `none` versus zero hexadecimal as different security truth is rejected; the ACL normalizes only equivalent empty-capability representations.
- Treating a host-built default Cargo binary as the immutable cross-image runtime artifact is rejected; release packaging must prove an architecture-compatible static artifact and bind its digest.

## Next causal RED

The next test must move from backend feasibility and source packaging to the production adapter boundary. Before changing `RootlessPodmanAdapter::run_command_at`, add an integration regression that requires it to:

- create the runtime-owned gate rather than the consumer argv as the initial OCI program;
- bind an immutable gate digest and architecture-compatible delivery artifact to the exact acquired container ID;
- keep the consumer unreleased while effective seccomp/capability/LSM evidence is sampled and policy-evaluated;
- reject absent/malformed evidence, wrong gate identity, unavailable gate delivery, release-channel failure, policy mismatch, or exec failure before consumer code can run;
- release exactly once and preserve the exact consumer argv plus existing no-network, exact-mount, resource, timeout, output and exact-ID lifecycle contracts;
- make the original payload-side-effect RED GREEN on the same implementation;
- preserve the real rootless production-gate E2E and separately acquire dedicated positive-LSM acceptance.

A fake integration test is appropriate for deterministic negative-path ordering, but it cannot replace the already-required real rootless and positive-LSM E2E.

## Release effect

No version, tag, package, GitHub Release, immutable consumer publication, protected merge, or descendant release claim is authorized while issue #25 lacks same-head production hold/attest/release GREEN, dedicated positive-LSM evidence, qualifying review/security gates, and protected-head integration evidence.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Podman Authors. (2026). *podman-top — Display the running processes of a container*. https://docs.podman.io/en/v4.9.3/markdown/podman-top.1.html

Kerrisk, M. (Ed.). (2026). *seccomp(2) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/seccomp.2.html

Kerrisk, M. (Ed.). (2026). *PR_SET_NO_NEW_PRIVS(2const) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/PR_SET_NO_NEW_PRIVS.2const.html

Kerrisk, M. (Ed.). (2026). *capabilities(7) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man7/capabilities.7.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
