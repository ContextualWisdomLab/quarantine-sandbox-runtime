# ADR 0006: Rootless isolated application-service profile

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

Chat/Agent systems sometimes need a real application process rather than an in-process tool function. Running such applications on the orchestrator host with ambient credentials, host networking, writable filesystem, or container-engine access creates an unacceptable authority boundary.

## Decision

The first `application_service` infrastructure adapter is rootless Podman. A valid request must use an immutable OCI image digest and bounded service/resources. The adapter:

- verifies Podman reports rootless mode;
- never pulls during launch (`--pull=never`);
- creates a unique internal DNS-disabled network;
- creates a read-only-root container;
- disables implicit writable read-only tmpfs and adds only an explicit bounded `/tmp` tmpfs with `noexec,nosuid,nodev`;
- drops all capabilities and enforces `no-new-privileges`;
- uses automatic user-namespace isolation and a numeric non-root UID/GID;
- uses private PID, IPC, UTS, and cgroup namespaces;
- disables restart and container logging in the P0 profile;
- applies CPU/RAM/PID and container lifetime limits;
- publishes one service port to random host port on `127.0.0.1` only;
- performs a bounded readiness check before returning a lease;
- removes container and network on explicit termination and on partial-launch/readiness failures;
- returns an attested versioned lease and cleanup receipt.

The application request has no fields for privileged mode, host namespaces, devices, arbitrary mounts, environment variables, runtime sockets, or external network enablement.

## Alternatives

- **Consumer-owned Podman calls:** rejected because isolation policy would be duplicated and domain code would depend on infrastructure.
- **Docker socket sidecar:** rejected because the socket is a high-authority control channel.
- **gVisor first:** deferred. gVisor is a planned stronger OCI backend but P0 first establishes the consumer-neutral contract and rootless lifecycle with broadly available Podman.
- **Kubernetes first:** deferred because a cluster is not required for standalone/local operation.

## Verification rule

Command-plan and fake-Podman process tests are necessary but not sufficient. Release claims about container isolation require a real rootless Podman E2E lane that checks effective filesystem, namespace, capability, resource, network, and cleanup behavior. gVisor/containerd/Kubernetes adapters require their own parity and security evidence.
