# Command Gate Composition Traceability

Status: active release-blocking trace for issue #25 / PR #14.

## Causal RED

Exact head `9d8fc6ff2256c4b0b4edd40f100f51691788368f`, native CI `34474788787`, verify job `102862937803`, passed repository policy and formatting before compiling `tests/podman_runtime_gate_command_integration_red.rs`. Compilation failed with Rust E0599 because `RuntimeGatePodmanAdapter` exposed no `run_command_at` operation. The executable regression therefore proved that the already-tested gate binding and release primitives were not composed into the canonical command lifecycle.

This RED is intentionally stronger than checking create argv alone. Its acceptance contract requires the verified gate binding to replace hostile consumer argv as OCI PID 1, live effective process isolation (`podman top`) to occur before the exact-ID one-time release channel, and completion evidence to be collected only after the trusted gate acknowledges release.

## Minimal production repair

Commit `0c5b11773bc95758981ab42880679ad8a605800d` composes the existing primitives without weakening their contracts:

- `RootlessPodmanAdapter` factors its command lifecycle through a shared internal runner that accepts either the historical direct-consumer binding or an already-verified runtime-gate binding.
- `RuntimeGatePodmanAdapter::run_command_at` creates the binding plan, delegates the common Podman lifecycle, and invokes `release_command_gate` only after static configuration and live seccomp/capability/LSM evidence have passed for the exact acquired container ID.
- `RuntimeGatePodmanAdapter` implements the `CommandExecutionBackend` application port so release-authorized callers can use the same bounded command use case without source-copying infrastructure logic.
- The temporary source-repair workflow removed itself in the same commit after applying the reviewed patch.

The required gated order is:

`verified immutable gate -> create gate as OCI PID 1 -> exact ID -> init/configuration contradiction checks -> start gate only -> live effective process attestation -> exact-ID one-time release -> trusted pre-exec ACK -> detached consumer stdin -> exact consumer exec -> bounded wait/logs -> exact-ID cleanup`

## Remaining owner-path gap

This commit is not issue #25 completion by itself. `RootlessPodmanAdapter` still retains the historical direct-consumer `CommandExecutionBackend` implementation and the CLI still constructs that adapter directly. Until the production CLI/owner path requires an independently supplied runtime-gate artifact plus expected SHA-256 and routes through `RuntimeGatePodmanAdapter`, the unsafe legacy bypass remains reachable and no containment or release claim is valid.

Acceptance therefore requires exact-head GREEN for the gated integration regression plus migration of the production owner path, removal or fail-closed retirement of the direct command trait bypass, dedicated positive-LSM evidence, full repository/fmt/test/Clippy/rustdoc/coverage/security gates, SBOM/provenance/reproducibility, and rollback evidence before protected integration or immutable publication.