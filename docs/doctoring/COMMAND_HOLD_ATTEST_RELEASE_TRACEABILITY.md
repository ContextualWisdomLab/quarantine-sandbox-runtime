# Command hold/attest/release traceability

Status: **Proposed / P0 open / backend capability, packaged-gate, bounded release-control, and trusted-ack candidate implemented**

This note records the current evidence and next acceptance boundary for issue #25. It does not mark ADR-0008 Accepted and it does not authorize a release.

## Causal P0 evidence

Canonical command-runtime PR #14 first executed `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` causally on exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`. Verify `102428599037` and branch coverage `102428599228` reached the same counterexample after preceding command-runtime/application-service contracts passed: production made hostile consumer argv the OCI program released by `podman start`, the fake backend recorded a payload side effect, and only then did effective-process attestation fail. Cleanup cannot undo that execution.

The historical production order was:

`create(consumer argv) -> acquire exact ID -> podman init -> configured-state verification -> podman start -> live process verification`

`podman init` is a useful earlier hold point, but it does not make the future consumer process's effective seccomp/LSM/capability state observable. Static/configured-state checks can reject contradictions before release; they cannot be relabeled as effective-process evidence.

## Real rootless backend capability proof

Exact `7da7d3b36ec8830ab16d776e5ed1a93f7a16b8ad`, native CI `34379177369`, hosted negative rootless/AppArmor job `102559489017` completed GREEN on Ubuntu 24.04 with rootless Podman 4.9.3. A runtime-owned static gate was mounted read-only at `/qsr-runtime-gate`, remained the only running process while the consumer sentinel was absent, exposed effective seccomp/capability/security-label evidence, preserved its SHA-256, and `exec()`ed the exact consumer argv only after explicit release.

The E2E also established two backend representation constraints rather than hiding them with retries. Podman 4.9.3 supports `capeff`, `capbnd`, `capinh`, `capprm`, `hpid`, `label`, and `seccomp` through `podman top`, but not `capamb`; ambient capability evidence therefore comes from `/proc/<hpid>/status`. Empty capability sets may be rendered as `none`, so the ACL normalizes only equivalent empty-set representations while rejecting non-empty sets.

This closes backend primitive feasibility only. Hosted negative LSM evidence does not substitute for a dedicated positive effective-LSM lane.

## Packaged production gate: RED -> focused GREEN

Test-only exact `304d63c2722a35b9f05f8466bbea81492a1533a6`, native CI `34381329199`, verify `102566637062`, passed repository/fmt prerequisites and then failed `command_hold_gate_packaging_red::unix::package_exposes_fail_closed_runtime_gate_with_exact_consumer_argv` because Cargo exposed no `CARGO_BIN_EXE_qsr_runtime_gate`.

Minimum production exact `3bd5d4319aba7c23982bbd18c77d424c25c3a510` added `src/bin/qsr_runtime_gate.rs`. The gate accepts one non-empty release token bounded to 128 bytes and the exact consumer program/argv, announces `QSR_GATE_READY`, reads one bounded token, rejects invalid invocation/channel/token fail closed, and uses Unix `exec()` so shell joining cannot alter the authorized argv. Native CI `34381498621` made the packaging regression GREEN in verify `102567695133` and statement coverage `102567694962`, then continued to the unchanged original #25 pre-attestation RED. Hosted negative rootless/AppArmor `102567695588` was GREEN.

Follow-up `a65381d50013f42c91b796e71755048ab3b7c584` made the real held-gate E2E compile production `src/bin/qsr_runtime_gate.rs` as `x86_64-unknown-linux-musl`; `013aeade4ae3a7fa86484c82b4bc9336c0e7e4ad` removed the duplicate test fixture. Later `RuntimeGateArtifact` work verifies an independently supplied expected lower-case SHA-256 and host/ELF architecture, rejects symlink/non-regular sources, stages into a private parent, and exposes non-writable user-namespace-readable executable bytes. `RuntimeGatePodmanAdapter::plan_command_binding` binds that artifact read-only at `/qsr-runtime-gate`, makes the gate the OCI entrypoint, generates a fresh 256-bit one-time release token, preserves exact consumer argv behind the gate, and keeps stdin open with `--interactive`.

## Controller-owned bounded release: causal RED -> production prerequisite

Exact `3fc5898d240a59add3d6c684fa324cec98c4e786`, native CI `34406027344`, executed `tests/podman_runtime_gate_release_control_red.rs` causally after repository/fmt prerequisites. Compilation failed with E0599 because `RuntimeGatePodmanAdapter::release_command_gate` did not exist.

Production descendant `e4acc2a7118900f7e60b73f488e7ba6461953c56` supplies the bounded release primitive. It admits only an exact 64-lowerhex acquired container ID, spawns `podman attach --sig-proxy=false <exact-id>`, writes this plan's one-time token once, closes the attach stdin, waits only for a bounded trusted acknowledgement, and then terminates/reaps the local attach client rather than the container. Spawn/write/read/wait/timeout/detach failures remain typed infrastructure failures. The plan is consumed by value so it is not a clonable second release authority.

Podman documents that interactive container stdin remains blocked while detached and is piped to the contained process when later attached. It also warns that Podman consumes available stdin immediately even if the contained process has not requested it. Therefore successful host-side write is not proof that the trusted gate accepted the token; a gate-originated pre-exec acknowledgement is required.

## Trusted gate acknowledgement: causal RED -> minimum candidate

Review of the controller release primitive exposed an independent protocol contradiction. `RuntimeGatePodmanAdapter::release_command_gate` requires `QSR_GATE_RELEASED\n`, but the trusted `qsr_runtime_gate` previously emitted only `QSR_GATE_READY\n`, accepted an exact token, and immediately replaced itself with consumer code. That made the controller's acknowledgement check non-authoritative: after becoming runnable, a hostile consumer sharing stdout could emit the expected marker itself.

A real-gate regression was added and later reordered test-only so Cargo would execute it before the broader hold-gate integration RED. Exact `a949fb2c653a70613bf26364b6e7993db8e5a38e`, native CI `34420060582`, verify `102693306116` passed exact checkout, dependency lock, repository policy, coverage-parser tests, rustfmt, 36 library tests, application-service/cleanup/primary command-runtime suites, and the exact ENTRYPOINT regression. It then executed `podman_command_execution_gate_ack_red::linux::runtime_gate_acknowledges_release_before_exec` and failed for the intended cause: after `QSR_GATE_READY\n` and exact token delivery the second line was empty instead of `QSR_GATE_RELEASED\n`. This is the causal trusted-gate acknowledgement RED.

Minimum production commit `0ebb9014226c546994817b4830ec9d06cb26c944` changes only the trusted gate. After exact token equality, it writes and flushes `QSR_GATE_RELEASED\n`; either write or flush failure exits through the existing `EXIT_CONTROL_CHANNEL_FAILED` path; only successful acknowledgement is followed by `exec()` of the exact consumer program/argv. READY semantics, token byte bounds, wrong-token rejection, and argv preservation are unchanged. Native CI `34423924325` materialized for that exact source candidate; predecessor checks do not transfer.

This acknowledgement is intentionally emitted by the trusted gate before the `execve` transition. It is not an assertion that the consumer completed safely, and it does not by itself close issue #25.

## Primary authority

OCI distinguishes `created` from `running`: in the created state the container process exists but the user-specified program has not yet executed; start transitions execution to the user-specified program. Portable code therefore cannot assume every process-specific security property is final merely because a container has been created or initialized.

Podman documents `podman init` as preparing a container for start without starting it. Podman `start` states that a detached interactive container blocks on stdin until later attached, while input is consumed as soon as available. Podman `attach` provides the exact container ID/name attachment surface and `--sig-proxy=false` prevents signals received by the local attach client from being proxied to the container. Those semantics support a bounded controller channel but do not themselves authenticate release; the gate-originated marker supplies that protocol evidence.

Linux preserves seccomp filters across `execve(2)` when `execve` is permitted. `no_new_privs` cannot be unset and is preserved across `execve(2)`. The capability bounding set constrains capabilities that can be gained during `execve(2)`. The real rootless E2E demonstrates that these properties can be sampled while a runtime-owned gate is held before the exact consumer exec transition.

## Selected production boundary

The selected architecture remains a **runtime-owned execution gate** as the OCI user process:

1. Bind the exact acquired container ID to an immutable architecture-compatible runtime gate instead of hostile consumer argv.
2. Start the gate under the final container process controls while it remains blocked before executing consumer code.
3. Observe effective seccomp, capability and LSM evidence for that exact running gate through the backend's live process evidence path.
4. Evaluate the selected isolation policy against bounded evidence.
5. Release only after positive evaluation through the exact-ID bounded controller channel.
6. Require `QSR_GATE_RELEASED` from the trusted gate after token validation and before the gate replaces itself with the exact consumer argv via `execve`.
7. Treat gate delivery, identity, evidence, release-channel, acknowledgement, exec, wait/log, or cleanup failure as fail closed with exact-ID lifecycle authority.

The real E2E, packaged gate, verified artifact, gate-binding plan, bounded controller release and trusted-ack candidate make every component of the selected mechanism concrete. Production acceptance still requires composing them in canonical `RootlessPodmanAdapter::run_command_at` and making the original hostile payload-side-effect RED GREEN on the same implementation.

## Rejected shortcuts

- Moving static `container inspect` earlier is insufficient because configured state is not the same as effective process state.
- `podman init` plus static evidence is only a pre-release contradiction check; it is not full effective attestation.
- Starting the hostile consumer and failing after `podman top` is too late; cleanup cannot undo execution.
- Plain `podman exec` after an inert hostile-image entrypoint does not establish a trusted gate and creates another executable authority surface.
- A shell/script or helper supplied by the hostile image cannot be the attestation authority.
- A `startContainer` hook alone is not accepted as proof of the later consumer process's exact effective state.
- Successful host-side stdin write is not release proof; Podman can consume input before the target process requests it.
- An acknowledgement emitted after consumer `exec` is not gate-originated proof and can be forged by hostile consumer stdout.
- Retry, sleep, mutex serialization or weakened assertions are not fixes for the release-order defect.
- Treating Podman display spelling such as `none` versus zero hexadecimal as different security truth is rejected; the ACL normalizes only equivalent empty-capability representations.
- Treating a host-built default Cargo binary as the immutable cross-image runtime artifact is rejected; release packaging must prove an architecture-compatible static artifact and bind its digest.

## Next causal integration boundary

The remaining P0 is now composition, not invention. Existing production still has a direct `RootlessPodmanAdapter::run_command_at` path that serializes consumer argv into OCI `--entrypoint`, calls `podman start`, and performs `verify_command_isolation` only afterwards. The already-checked-in hold-gate binding/pre-attestation regressions must therefore remain RED until that canonical method owns the selected mechanism.

The next production repair must preserve all existing command isolation/resource/source/output/lifecycle contracts while composing:

- independently verified `RuntimeGateArtifact` identity and architecture;
- gate create args as the OCI entrypoint, with exact consumer argv behind the one-time token;
- exact acquired container ID as the sole post-create lifecycle/destructive authority;
- pre-start configuration contradiction checks;
- gate-only start;
- effective seccomp/capability/LSM attestation while consumer code remains non-runnable;
- positive policy decision before release;
- one-time exact-ID bounded release and trusted pre-exec acknowledgement;
- exact consumer argv execution, bounded wait/log collection, and cleanup;
- fail-closed negative paths for absent/malformed effective evidence, artifact mismatch, wrong token/EOF/channel failure, acknowledgement failure, timeout/cancellation and cleanup failure;
- the original pre-attestation payload-side-effect regression GREEN on the same implementation;
- retained real rootless held-gate E2E and independent dedicated positive-LSM acceptance.

A fake integration regression is appropriate for deterministic negative-path ordering, but it cannot replace the real rootless and positive-LSM E2E.

## Release effect

No version, tag, package, GitHub Release, immutable consumer publication, protected merge, or descendant release claim is authorized while issue #25 lacks same-head production hold/attest/release GREEN, complete owned-production/branch coverage, dedicated positive-LSM evidence, qualifying review/security gates, and protected-head integration evidence.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Podman Authors. (2026). *podman-attach — Attach to a running container*. https://docs.podman.io/en/latest/markdown/podman-attach.1.html

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Podman Authors. (2026). *podman-start — Start one or more containers*. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Podman Authors. (2026). *podman-top — Display the running processes of a container*. https://docs.podman.io/en/v4.9.3/markdown/podman-top.1.html

Kerrisk, M. (Ed.). (2026). *seccomp(2) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/seccomp.2.html

Kerrisk, M. (Ed.). (2026). *PR_SET_NO_NEW_PRIVS(2const) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/PR_SET_NO_NEW_PRIVS.2const.html

Kerrisk, M. (Ed.). (2026). *capabilities(7) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man7/capabilities.7.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
