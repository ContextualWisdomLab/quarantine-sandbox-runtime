# Command Namespace Traceability

## Decision

The command-runtime profile requests private UTS and cgroup namespaces with `podman create --uts=private --cgroupns=private`. Those flags are launch intent, not sufficient evidence that the created hostile-workload container actually received the reviewed namespace modes. Before command output is trusted, the Podman adapter must bind the backend-applied `HostConfig.UTSMode` and `HostConfig.CgroupMode` values to that request and fail closed on host/shared namespace modes.

This decision implements the repository security invariant that isolated namespaces are P0 and host namespaces are forbidden. It does not relabel Podman inspect as live kernel enforcement. Real release evidence still needs a rootless runtime check of the running process namespace handles (or an equivalent authoritative primitive) in addition to backend-applied configuration.

## Why these two fields matter

Podman documents `--uts=private` as creating a new UTS namespace and `--cgroupns=private` as creating a new cgroup namespace. `podman container inspect` exposes the applied modes as `HostConfig.UTSMode` and `HostConfig.CgroupMode`.

Linux UTS namespaces isolate hostname and NIS domain identifiers. Linux cgroup namespaces virtualize cgroup paths visible through `/proc`, including hiding ancestor cgroup hierarchy information that could otherwise leak container-runtime or host topology. NIST SP 800-190 treats namespace isolation as a container-runtime security control rather than a launch-argument convention.

## Alternatives considered

Relying on the create argv was rejected because it proves configured intent only. Relying solely on default Podman behavior was rejected because defaults vary with runtime and cgroup version, and this profile already requests explicit private modes. Treating successful rootless/user/PID/IPC/network checks as proof for UTS/cgroup isolation was rejected because those are independent namespaces. Treating inspect fields as final live-enforcement proof was also rejected; they are an applied-state gate that must later be paired with real runtime evidence.

## Verification chain

Issue #39 owns the missing applied-state binding. Test-bearing RED `eac6b8afe998cc34171869717b882bba4002b618` adds `tests/podman_command_execution_namespace_config_red.rs`. The fixture keeps every currently checked command-isolation fact positive while independently reporting `UTSMode=host` and `CgroupMode=host`; each case requires a precise fail-closed namespace control, no trusted `podman logs` read, and exact invocation-owned cleanup.

Production remains intentionally unchanged until that RED executes for the intended missing-attestation cause. The smallest causal GREEN is to deserialize both inspect fields and require the exact private modes requested by the runtime. That GREEN must not weaken issue #25 pre-payload attestation, issue #36 lifecycle ownership, or any live seccomp/LSM/capability requirement.

## References

Linux man-pages project. (2026). *cgroup_namespaces(7)—Overview of Linux cgroup namespaces*. https://man7.org/linux/man-pages/man7/cgroup_namespaces.7.html

Linux man-pages project. (2026). *uts_namespaces(7)—Overview of Linux UTS namespaces*. https://man7.org/linux/man-pages/man7/uts_namespaces.7.html

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
