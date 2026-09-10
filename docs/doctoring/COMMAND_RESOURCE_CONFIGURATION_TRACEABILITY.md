# Command resource configuration traceability

## Decision scope

Issue #35 owns the command-execution boundary that distinguishes **requested resource controls**, **backend-applied configuration**, and **live kernel/runtime enforcement**. The canonical command path emits `--timeout <lease_seconds>` and `/tmp:rw,noexec,nosuid,nodev,size=<tmpfs_bytes>` at create time. Those arguments are intent, not evidence. A command is not allowed to advance from pre-start configuration verification merely because the requested flags were present.

The causal RED was executed on PR #14 exact `97dff877a243f7d772d02e9ca2cf93f470b7fe2f`, native CI `34461059296`, verify `102818736736`. `tests/podman_command_execution_resource_config_red.rs` supplied otherwise-positive inspection data while independently varying three backend-applied facts: an executable/widened `/tmp`, `Config.Timeout=0`, and `Config.Timeout=21` for a 20-second request. Production false-GREENed all three as successful command executions because `ContainerInspection` did not deserialize `Config.Timeout` or `HostConfig.Tmpfs` and the command verifier delegated only to the generic CPU/RAM/PID comparison.

## Minimum production repair

Production commit `dd7242e53a79f1a8004e24010ae999fda70e78af` adds command-specific applied-configuration verification without changing the application-service verifier:

- `Config.Timeout` is deserialized and must be non-zero and exactly equal to `CommandExecutionRequest.resources.lease_seconds`.
- `HostConfig.Tmpfs` is deserialized and must contain exactly one destination, `/tmp`.
- `/tmp` must contain exactly the five request-bound options `rw`, `noexec`, `nosuid`, `nodev`, and `size=<tmpfs_bytes>`; missing, duplicate, additional, contradictory, or wrong-size options fail closed.
- Existing positive RAM, CPU, and PID bounds remain prerequisites.
- Missing inspection fields deserialize to zero/empty and therefore fail closed on the command path instead of being interpreted as compatible evidence.

Test-fixture descendants `75df58cd2753eaea64fa62ad3557dd6caf19cbf5` and `4740624d2ac930a1e87d60bc0be0e45135d5c040` migrate otherwise-positive fake Podman inspections to the now-authoritative timeout/tmpfs facts. They do not weaken the #35 RED assertions.

## Evidence classification

`container inspect` is accepted here only as evidence of **backend-applied configuration**. It does not prove the effective mount flags observed by the workload, cgroup-v2 CPU/memory/PID enforcement, or behavioral termination at the lease boundary. Release acceptance therefore still requires real rootless-Podman evidence for the effective `/tmp` mount, cgroup controls, wall-time termination, and exact-ID cleanup. The same distinction applies to issue #25: pre-start configuration contradiction checks are useful, but they cannot replace effective-process seccomp/capability/LSM attestation before consumer release.

Podman documents `--timeout` as a maximum container run time after which conmon sends a kill signal, and `--tmpfs` as a tmpfs mount whose mount options are Linux mount flags; the documented default set includes `rw,noexec,nosuid,nodev`. The OCI Runtime Specification separately distinguishes the `created` state from `running`: the user-specified program has not executed in `created`, while `start` applies `process.args`. That lifecycle distinction is why applied configuration must be rejected while the workload remains held rather than discovered only after consumer execution. NIST SP 800-190 treats runtime configuration and isolation as container-security control concerns; it is used as control rationale, not as evidence that a particular Podman instance enforced the requested values.

## Reproducible acceptance

The focused source repair is GREEN only when an unchanged exact head demonstrates all of the following:

1. the three existing #35 counterexamples fail closed with `IsolationVerificationFailed { control_name: "resource_limits" }` and exact acquired-ID cleanup;
2. otherwise-positive command fixtures progress to their intended later lifecycle assertions after supplying authoritative `Timeout` and `Tmpfs` facts;
3. missing, duplicate, extra-destination, contradictory, wrong-size, zero, and request-mismatched resource inspection evidence are covered without accepting a wider configuration;
4. real rootless-Podman acceptance separately proves effective `/tmp` mount flags/size, cgroup-v2 CPU/RAM/PID behavior, wall-time termination, and leak-free cleanup;
5. issue #25 remains independently RED until the runtime-owned gate is the only OCI initial program through hold → effective attestation → one-time release → trusted acknowledgement → consumer `exec`.

## References

National Institute of Standards and Technology. (2017). *Application container security guide* (NIST Special Publication 800-190). https://doi.org/10.6028/NIST.SP.800-190

Open Container Initiative. (n.d.). *Open Container Initiative Runtime Specification: Runtime and lifecycle*. Retrieved September 10, 2026, from https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Podman. (2026). *podman-create — Podman documentation* (Version 5.6.0). https://docs.podman.io/en/v5.6.0/markdown/podman-create.1.html
