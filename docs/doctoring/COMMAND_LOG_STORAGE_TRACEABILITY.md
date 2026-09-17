# Command log-storage traceability

## Decision under review

A bounded command must not be able to consume unbounded host storage through the container runtime's own log file while the workload is running. The later `podman logs` reader budget protects the control process from unbounded retained stdout/stderr; it is not a host-disk quota for the runtime-managed `k8s-file` log.

## Causal RED

The hostile regression is `tests/podman_command_execution_log_storage_red.rs`, strengthened by test-bearing commit `0add9f117f39f78b04526d4c775476b3587ae965` to require exactly one positive finite `--log-opt max-size=<value>` and reject empty, zero, malformed, duplicate, or effectively unbounded forms.

On canonical PR #14 exact predecessor `72053f437a1a834e79f8db5f9d8701abc3d1a0e6`, native CI `34261831371` verify job `102181436826` checked out the exact SHA, passed dependency locking, repository validation, coverage-parser tests and `cargo fmt --check`, and then ran the Rust suite. The #38 exact-ENTRYPOINT, #28 >=128-bit runtime-identity, and #37 supplied-mismatch image-digest regressions all passed. The suite then failed exactly at `podman_command_execution_log_storage_red::command_container_requires_one_positive_finite_runtime_log_file_limit`: fake Podman rejected command-container creation because the `k8s-file` launch argv contained no finite runtime-owned `max-size` option.

## Minimum finite-storage candidate

Commit `789a0f1f14dec1b5c743411fd45f19cd467771d5` introduces a backend-owned `DEFAULT_COMMAND_LOG_STORAGE_LIMIT_BYTES` of 1 MiB and passes exactly one `--log-opt max-size=1048576b` beside `--log-driver=k8s-file` for command containers. Application-service containers remain on `--log-driver=none`; the change is confined to the command-execution infrastructure adapter.

The 1 MiB cap is intentionally larger than the default 64 KiB retained-output limit for each stream, so normal bounded receipts are not forced through a host-log cap before the reader limit. It is still a finite host-storage boundary, not proof of complete output after a hostile log flood.

## Incompleteness boundary

This candidate does **not** complete issue #31. A finite `k8s-file` cap can cause runtime-side loss, rotation, or truncation before `podman logs` reads the file. The existing `stdout_truncated` / `stderr_truncated` flags describe the bounded reader's own retained-output limit; they are not yet positive evidence that Podman did not discard earlier records. Returning an apparently complete receipt after runtime-side loss would be false evidence.

Before #31 can close or contribute release-grade output attestation, a real rootless-Podman log-flood E2E must demonstrate the actual `max-size` behavior, prove host log growth stays within the declared cap, preserve timeout and exact cleanup behavior, and surface runtime-side incompleteness explicitly whenever the log store loses data. Increasing the cap without such evidence, redirecting logs to an unbounded host path, or dropping output silently are rejected alternatives.

## DDD boundary

The storage cap is infrastructure policy inside the command-execution adapter. The command request does not gain authority to widen or disable it. `CommandExecutionResult` remains application-service evidence and must not claim complete output unless both the runtime log store and the bounded reader can support that claim. Host storage resource isolation is separate from CPU/RAM/PID/tmpfs/time limits and from #34's lossless byte-encoding contract.

## References

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/stable/markdown/podman-create.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
