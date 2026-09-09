# Command hold-gate binding traceability

Status: active Draft evidence for issue #25 and ADR-0008. This document does not authorize release.

## Problem

`RootlessPodmanAdapter::run_command_at` still serializes the requested consumer argv into Podman's OCI entrypoint during `create`. `podman start` can therefore make hostile consumer code runnable before the runtime has positively sampled effective seccomp, capability and LSM state. Cleanup after a failed attestation does not undo code execution.

The package already contains the runtime-owned `qsr_runtime_gate`, and the dedicated real-rootless capability test has shown that the gate can remain the sole running process, expose effective process evidence and `exec()` exact consumer argv after explicit release. The missing production boundary is now decomposed so gate artifact authority, gate delivery/initial-process binding, stdin control-channel liveness and later release-channel ownership can be proven independently before they are integrated into the canonical command lifecycle.

## Causal evidence

- Original hostile payload-side-effect RED: exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, verify `102428599037` and branch coverage `102428599228`. The payload became runnable at `start` before live attestation failed.
- Packaged gate capability: minimum production `3bd5d4319aba7c23982bbd18c77d424c25c3a510` introduced `src/bin/qsr_runtime_gate.rs`; exact CI `34381498621` made the focused packaging regression GREEN before the unchanged pre-attestation RED stopped the broader run.
- Real held-gate capability: the dedicated rootless-Podman test compiles the production gate source as a static `x86_64-unknown-linux-musl` executable, mounts it read-only at `/qsr-runtime-gate`, creates the container with interactive stdin, verifies the gate is the only process before release, samples seccomp/capability/ambient/label evidence, then releases exact consumer argv through `podman attach --sig-proxy=false`.
- Gate-binding RED: test-only `98b75125a619c981086ec1babb07fbc298ca7601` added `tests/podman_command_execution_hold_gate_binding_red.rs`. Native CI `34393277076` stopped at rustfmt before semantic execution; formatter-only `5a591a582ca955ccd24b8666a779de279062fae7` changed no assertion or production behavior.
- Exact `5a591a582ca955ccd24b8666a779de279062fae7`, native CI `34393466321`, verify `102607270660`, then passed repository policy, dependency lock, coverage-parser checks, rustfmt and all preceding command-runtime tests through the exact ENTRYPOINT regression. It failed `podman_create_binds_runtime_gate_as_initial_process_before_consumer_argv` for the intended cause: the observed `create` invocation still contained `--entrypoint=["payload-sentinel","argument with spaces"]` and no runtime-owned `/qsr-runtime-gate` delivery/binding.
- Intervening owner work then introduced `RuntimeGateArtifact` plus `RuntimeGatePodmanAdapter::plan_command_binding`: an independently digest- and architecture-verified staged gate is bound read-only at `/qsr-runtime-gate`, selected as OCI entrypoint, and receives a fresh 256-bit one-time release token plus exact consumer argv. This is a planning boundary only; it does not claim that `run_command_at` has adopted the gate.
- Stdin-liveness RED: test-only `9dc5b7740a26322891715c84902170e3b411a735`, formatted by `0ec969b70d647eef9c69149875070136648b21e7`, requires the planned container to keep stdin open. Native CI `34402406692`, verify `102637216130`, passed exact checkout, dependency lock, repository policy, coverage-parser checks and rustfmt, then executed `gate_binding_keeps_container_stdin_open_for_bounded_release` for the intended cause: observed first create argument `--volume`, required `--interactive`; 35 other unit tests passed.
- Minimum stdin-liveness production repair `b35fce53527c872abd00cffb34b0b831cfa7af74` adds only `--interactive` to the gate-binding create fragment. Test-only `d6a3078e5b9b1983b72101aacfb4111b110fc501` aligns the independent artifact-binding regression with that required argv order.
- Exact `d6a3078e5b9b1983b72101aacfb4111b110fc501`, native CI `34402589899`, verify `102638265681`, proves the stdin-liveness repair focused GREEN: all 36 library unit tests pass, including `gate_binding_keeps_container_stdin_open_for_bounded_release`; repository policy, dependency lock, coverage-parser checks and rustfmt also pass. The broader verify lane later fails an inherited fake-Podman process-spawn specimen in `podman_cleanup_regression` (`BackendSpawnFailed { operation: "backend_security_info", failure_kind: Other }` versus the fixture's intended `CleanupFailed`). This is not promoted to whole-head GREEN, and no retry or semantic weakening is authorized. Hosted negative rootless/AppArmor on the same run reaches and passes the real held-gate capability, unavailable-LSM fail-closed cleanup and leak-rejection steps. Dedicated positive-LSM remains a separate gate.

No predecessor GREEN transfers to a moved head. The evidence above proves the missing gate binding and stdin-liveness prerequisites independently; it does not prove a completed production hold/attest/release adapter.

## Why interactive stdin is a security prerequisite

`qsr_runtime_gate` reads the release token from its stdin before it can `exec()` the consumer. Podman's current `start` contract states that a non-interactive container has empty/closed stdin, while an interactive detached container can remain blocked until a later attach. Podman's `create -i/--interactive` contract likewise keeps stdin available for the container process. Omitting `--interactive` therefore creates a deterministic protocol contradiction: the gate can receive EOF and exit before controller-side attestation/release, even though the gate artifact and token syntax are otherwise correct.

This repair does **not** make stdin itself a trust anchor. Release authorization still requires a controller-owned bounded attach/write operation tied to the exact acquired container ID, exact one-time token, timeout, cancellation and cleanup semantics. The gate must fail closed on EOF, wrong token, duplicate/late release or channel failure, and none of those failures may permit the consumer argv to run.

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

### Detached gate without interactive stdin

Rejected by causal RED. The gate's one-time release protocol is stdin-based; a detached non-interactive Podman container can present EOF instead of a held control channel. The minimum create contract therefore includes `--interactive`, while later release remains controller-owned and exact-ID scoped.

## Minimum next source slice

The gate artifact and create-fragment prerequisites are now independently modeled, including required interactive stdin. The next causal source slice must add a bounded controller-owned release primitive before integrating it into `run_command_at`. That primitive must:

1. address only the exact acquired container ID;
2. write exactly one runtime-generated release token to the held gate without exposing a reusable interactive session to the consumer;
3. distinguish spawn/write/wait/timeout/cancellation failures without retry-based masking;
4. prove EOF/wrong-token/channel-failure cleanup leaves consumer argv non-runnable;
5. compose with existing bounded command supervision and cleanup precedence rather than creating an unbounded `podman attach` subprocess.

Only after those REDs execute may the canonical `run_command_at` lifecycle adopt the release-authorized `RuntimeGateArtifact`, create the interactive held gate as PID 1, perform static contradiction checks, start only that gate, attest effective seccomp/capability/LSM state, release once, and then wait/log/clean up by exact ID. Rerun the gate-binding RED and the original payload-side-effect RED on that same integrated exact head. Focused gate-plan GREEN without integrated release/effective attestation/positive-LSM evidence is not release authority.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Open Container Initiative. (2026). *Open Container Initiative runtime specification: POSIX-platform hooks*. https://github.com/opencontainers/runtime-spec/blob/main/config.md

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Podman Authors. (2026). *podman-start — Start one or more containers*. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Podman Authors. (2026). *podman — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
