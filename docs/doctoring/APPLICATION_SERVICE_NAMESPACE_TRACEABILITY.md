# Application-service namespace traceability

## Decision under review

The application-service P0 profile already requests `--uts=private` and `--cgroupns=private`. That launch intent is not sufficient isolation evidence. Before a service endpoint can be published or readiness can be accepted, the Podman anti-corruption layer must bind the backend-applied UTS and cgroup namespace modes to the request and reject host/shared modes.

This decision remains RED-only under issue #45. Production must not be changed until the checked-in hostile fixtures execute for the intended false-GREEN cause on their exact head.

## External basis

Podman documents `--cgroupns=private` as creation of a new cgroup namespace and `--uts=private` as creation of a private UTS namespace. Current `podman container inspect` output exposes the applied values as `HostConfig.CgroupMode` and `HostConfig.UTSMode`. Linux documents that UTS namespaces isolate hostname/domain identifiers, while cgroup namespaces virtualize the cgroup view visible through procfs. NIST SP 800-190 treats runtime isolation controls as container-security boundaries that require verification rather than trust in requested configuration.

References are maintained in `docs/doctoring/REFERENCES.md` in APA 7th style.

## Code-current gap

At test-bearing commit `d9c5b201e1e6548f29de438e7f470445118a5e64`, `RootlessPodmanAdapter::plan_at` requests both private namespaces, but `ContainerHostConfig` does not deserialize `UTSMode` or `CgroupMode` and `RootlessPodmanAdapter::verify_effective_isolation` checks user/PID/IPC namespaces only. An otherwise-positive inspection reporting `UTSMode=host` or `CgroupMode=host` can therefore continue to port lookup/readiness and become a lease.

`tests/podman_application_service_namespace_config_red.rs` is the causal RED. It holds rootless, seccomp, LSM, capabilities, user/PID/IPC, resource and network-object evidence positive while changing exactly one applied namespace mode to `host`. Each fixture requires a distinct fail-closed control (`isolated_uts_namespace` or `isolated_cgroup_namespace`), rejection before `podman port`, and cleanup of runtime-owned resources.

## Smallest causal GREEN after reproduced RED

1. Deserialize `HostConfig.UTSMode` and `HostConfig.CgroupMode` in the Podman inspection DTO.
2. Require exact `private` values in `verify_effective_isolation` before network port evidence or readiness is trusted.
3. Keep control names aligned with command-runtime issue #39 so both paths use one isolation vocabulary.
4. Do not promote inspect state to live enforcement evidence. Release-grade positive proof must additionally bind the exact running sandbox process to authoritative namespace handles (for example `/proc/<pid>/ns/uts` and `/proc/<pid>/ns/cgroup`) or an equivalent reviewed backend primitive.

## Alternatives rejected

- Relying on create argv only: rejected because requested configuration is not applied-state evidence.
- Treating `private` as optional because Podman defaults may already be private: rejected because the P0 contract explicitly forbids host namespaces and must fail closed if the backend reports otherwise.
- Reusing PID/IPC namespace checks as a proxy: rejected because Linux namespaces are independent isolation dimensions.
- Claiming `HostConfig` inspection proves kernel enforcement: rejected; it is a stronger configuration-state binding, not a live namespace-identity proof.

## Boundary map

Issue #45 is independent of #19 tmpfs/wall-time resource attestation, #20/#40/#42 invocation/lifecycle ownership, #22/#23/#41 network attachment/cleanup ownership, and command-runtime #39. The application-service domain remains backend-neutral; Podman-specific fields stay inside `src/infrastructure/podman.rs` and are translated into the existing fail-closed isolation vocabulary.
