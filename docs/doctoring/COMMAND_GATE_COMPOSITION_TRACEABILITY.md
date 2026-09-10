# Command Gate Composition Traceability

Status: active release-blocking trace for issue #25 / PR #14.

## Runtime composition causal RED

Exact head `9d8fc6ff2256c4b0b4edd40f100f51691788368f`, native CI `34474788787`, verify job `102862937803`, passed repository policy and formatting before compiling `tests/podman_runtime_gate_command_integration_red.rs`. Compilation failed with Rust E0599 because `RuntimeGatePodmanAdapter` exposed no `run_command_at` operation. The executable regression therefore proved that the already-tested gate binding and release primitives were not composed into the canonical command lifecycle.

The acceptance contract is stronger than checking create argv alone. The verified gate must replace hostile consumer argv as OCI PID 1, live effective process isolation must be sampled before exact-ID release, and completion evidence must be collected only after the trusted gate acknowledges release.

## Runtime composition repair

Commit `0c5b11773bc95758981ab42880679ad8a605800d` composes the existing primitives without weakening their contracts:

- `RootlessPodmanAdapter` factors its command lifecycle through a common internal runner that accepts either the historical direct-consumer binding or an already-verified runtime-gate binding.
- `RuntimeGatePodmanAdapter::run_command_at` creates the binding plan, delegates the common Podman lifecycle, and invokes `release_command_gate` only after configured-state and live seccomp/capability/LSM evidence pass for the exact acquired container ID.
- `RuntimeGatePodmanAdapter` implements the `CommandExecutionBackend` application port so release-authorized callers use the same bounded command use case without copying infrastructure logic.

The gated order is:

`verified immutable gate -> create gate as OCI PID 1 -> exact ID -> init/configuration contradiction checks -> start gate only -> live effective process attestation -> exact-ID one-time release -> trusted pre-exec ACK -> detached consumer stdin -> exact consumer exec -> bounded wait/logs -> exact-ID cleanup`

This ordering follows the OCI Runtime Specification 1.3.0 lifecycle: `create` establishes the runtime environment without running the user-specified program, while `start` transitions the configured process to execution. Podman likewise documents `podman create` as preparing but not starting the container and `podman start` as the operation that starts it. Podman also documents that detached interactive stdin blocks until a later attach and that available stdin can be consumed as soon as it arrives, which is why host-side write success is not release proof and the trusted pre-exec acknowledgement remains a separate invariant.

## Production CLI owner-path causal RED

Exact head `81f029e67a1b6df6f652e748b16337b85c7e590f`, native CI `34476087909`, branch-coverage job `102867072719`, executed `tests/command_cli_runtime_gate_owner_red.rs`. The regression launched the shipped `quarantine-sandbox-runtime run` command without runtime-gate trust inputs and supplied a fake Podman that records any spawn. The CLI did spawn Podman, causing the assertion `production CLI spawned Podman before independently verifying a runtime-gate artifact` to fail. This proved that the production transport could bypass the verified gate even after the gated infrastructure adapter existed.

## Production CLI owner-path repair

Commit `9e29fdc784885b1d755fe45b6530a1cf9d77c997` migrated the shipped CLI to the verified owner path, with the later type-corrected lineage retained by ordinary descendants:

- `run` requires `--runtime-gate-path` and `--runtime-gate-sha256` as independent artifact identity inputs.
- Missing or partial gate identity is rejected as a configuration error before Podman can be spawned.
- `RuntimeGateArtifact::stage` verifies supplied digest, host architecture and self-contained ELF admission before backend execution.
- Only the resulting `RuntimeGatePodmanAdapter` is passed to `execute_command`, so the shipped transport cannot select the historical direct-consumer adapter.

Exact head `58b79f4fa7f2146145b63975fffe2b3221936ac4`, native CI `34477279196`, verify job `102871027480`, established the gated integration and CLI-owner regressions as GREEN. That run failed only the two historical direct-`RootlessPodmanAdapter` hostile tests, proving that the remaining issue #25 defect had narrowed from the shipped CLI to the library/API bypass.

## Direct library bypass retirement

Commit `cbb82bc0c2b2f1e84c07bf4eb7528813f3a29cf9` removes `CommandExecutionBackend` from `RootlessPodmanAdapter` and compiles the historical direct-consumer executor only under `debug_assertions` as `run_legacy_command_at_for_test`. Existing low-level hostile and lifecycle tests were moved to that explicitly non-release test boundary. The obsolete hold-gate and pre-attestation regressions were removed only after their security semantics had been inherited by `podman_runtime_gate_command_integration_red` and `command_cli_runtime_gate_owner_red`.

The same repair also resolves two `-D warnings` Clippy blockers found while validating the retirement: release callback error propagation is flattened without changing exact-ID cleanup semantics, and release-client cleanup uses direct `?` propagation. The purpose-scoped source-repair workflow deleted itself in the successful source commit.

This is a release-API retirement, not a claim that debug builds are containment-certified. Debug artifacts retain the legacy executor specifically for direct infrastructure tests and must not be distributed as production runtime packages. Release acceptance should include an optimized/release compilation gate in addition to the normal debug test/Clippy/doc/coverage suites so absence of the legacy symbol is checked in the artifact configuration that will be published.

## Issue #35 relationship

Exact `300c91d74208fe01d7b5d08b26ca7b75913da6f7`, native CI `34473139307`, verify `102857579032`, established 8/8 GREEN for configured-state hostile resource cases: widened `/tmp`, zero/mismatched/missing timeout, missing/additional tmpfs destination, and duplicate/contradictory tmpfs options all fail closed with exact acquired-ID cleanup. This closes only configured-state false-GREEN behavior. Real rootless Podman evidence for effective `/tmp` mount flags/size, cgroup-v2 CPU/RAM/PID enforcement, behavioral wall-time termination, and leak-free cleanup remains a release gate.

## Remaining acceptance

Issue #25 can be considered source-composed only after an ordinary exact-head CI run proves the post-retirement tree: gated integration and CLI-owner regressions GREEN, all retained direct low-level tests GREEN under the non-release test boundary, no production `CommandExecutionBackend` implementation for `RootlessPodmanAdapter`, and repository/fmt/full-target/Clippy/rustdoc/coverage gates GREEN. Protected integration and immutable publication additionally require dedicated positive-LSM evidence, release-configuration compilation, security/review gates, SBOM/provenance/reproducibility, rollback evidence, and issue #35's real-runtime resource enforcement evidence.

## References

Open Container Initiative. (2025). *Open Container Initiative runtime specification* (Version 1.3.0). https://specs.opencontainers.org/runtime-spec/runtime/

Podman. (2026). *podman-create — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (2026). *podman-start — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman-start.1.html
