# Command Gate Composition Traceability

Status: active release-blocking trace for issue #25 / PR #14.

## Runtime composition causal RED

Exact head `9d8fc6ff2256c4b0b4edd40f100f51691788368f`, native CI `34474788787`, verify job `102862937803`, passed repository policy and formatting before compiling `tests/podman_runtime_gate_command_integration_red.rs`. Compilation failed with Rust E0599 because `RuntimeGatePodmanAdapter` exposed no `run_command_at` operation. The executable regression therefore proved that the already-tested gate binding and release primitives were not composed into the canonical command lifecycle.

This RED is intentionally stronger than checking create argv alone. Its acceptance contract requires the verified gate binding to replace hostile consumer argv as OCI PID 1, live effective process isolation (`podman top`) to occur before the exact-ID one-time release channel, and completion evidence to be collected only after the trusted gate acknowledges release.

## Runtime composition repair

Commit `0c5b11773bc95758981ab42880679ad8a605800d` composes the existing primitives without weakening their contracts:

- `RootlessPodmanAdapter` factors its command lifecycle through a shared internal runner that accepts either the historical direct-consumer binding or an already-verified runtime-gate binding.
- `RuntimeGatePodmanAdapter::run_command_at` creates the binding plan, delegates the common Podman lifecycle, and invokes `release_command_gate` only after static configuration and live seccomp/capability/LSM evidence have passed for the exact acquired container ID.
- `RuntimeGatePodmanAdapter` implements the `CommandExecutionBackend` application port so release-authorized callers can use the same bounded command use case without source-copying infrastructure logic.
- The temporary source-repair workflow removed itself in the same commit after applying the reviewed patch.

The required gated order is:

`verified immutable gate -> create gate as OCI PID 1 -> exact ID -> init/configuration contradiction checks -> start gate only -> live effective process attestation -> exact-ID one-time release -> trusted pre-exec ACK -> detached consumer stdin -> exact consumer exec -> bounded wait/logs -> exact-ID cleanup`

## Production CLI owner-path causal RED

Exact head `81f029e67a1b6df6f652e748b16337b85c7e590f`, native CI `34476087909`, branch-coverage job `102867072719`, executed `tests/command_cli_runtime_gate_owner_red.rs`. The regression launched the shipped `quarantine-sandbox-runtime run` command without runtime-gate trust inputs and supplied a fake Podman that records any spawn. The CLI did spawn Podman, causing the assertion `production CLI spawned Podman before independently verifying a runtime-gate artifact` to fail. This proves the production transport could still bypass the verified gate even after the gated infrastructure adapter existed.

Formatter-only descendant `fb69601ebbb291345c49b6b0b173e315b4ae84af` preserves that assertion and source semantics.

## Production CLI owner-path repair

Commit `9e29fdc784885b1d755fe45b6530a1cf9d77c997` migrates the shipped CLI to the verified owner path:

- `run` accepts `--runtime-gate-path` and `--runtime-gate-sha256` as the independent artifact identity inputs.
- Missing or partial gate identity is rejected as a configuration error before `RootlessPodmanAdapter` can spawn Podman.
- `RuntimeGateArtifact::stage` verifies the supplied digest, host architecture and self-contained ELF admission before backend execution.
- Only the resulting `RuntimeGatePodmanAdapter` is passed to `execute_command`, so the production CLI uses the hold/attest/release lifecycle rather than the historical direct-consumer adapter.
- CLI positive/backend-error fixtures now provide a self-contained gate fixture and exercise the gated application-port path.
- The temporary CLI repair workflow removed itself in the same source commit.

The source commit was produced by a purpose-scoped workflow using an ordinary non-force descendant push. Because a `GITHUB_TOKEN`-originated push does not provide the normal exact-head CI evidence required for acceptance, this traceability update intentionally creates a fresh ordinary branch head so the repository CI can validate the integrated source without treating the source-fix workflow as evidence.

## Remaining owner-path gap

The CLI migration closes the shipped transport bypass only if current exact-head tests prove it. The library still exposes the historical direct-consumer `RootlessPodmanAdapter` command implementation and its `CommandExecutionBackend` implementation for migration tests. That direct trait path must be removed, restricted, or made fail closed before the library itself can be treated as a release-authorized containment boundary.

Acceptance therefore requires exact-head GREEN for the gated composition and CLI-owner regressions, fail-closed retirement of the direct command trait bypass, dedicated positive-LSM evidence, full repository/fmt/test/Clippy/rustdoc/coverage/security gates, SBOM/provenance/reproducibility, and rollback evidence before protected integration or immutable publication.