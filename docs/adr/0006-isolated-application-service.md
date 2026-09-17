# ADR 0006: Rootless isolated application-service profile

- **Status:** Proposed
- **Date:** 2026-09-01

This ADR remains Proposed while PR #1 is Draft and the exact candidate has not produced the required current-head real rootless Podman isolation/cleanup evidence. Promote it to Accepted only after the decision is integrated into the protected branch and revalidated against the then-live repository/security governance.

## Context

Chat/Agent systems sometimes need a real application process rather than an in-process tool function. Running such applications on the orchestrator host with ambient credentials, host networking, writable filesystem, or container-engine access creates an unacceptable authority boundary.

A loopback port accepting TCP is not sufficient evidence that an application declared as HTTP is ready for a consumer security bootstrap. The runtime therefore needs protocol-aware readiness while keeping consumer-specific login, authorization, and arbitrary health URLs outside the isolation bounded context.

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
- performs protocol-aware bounded readiness before returning a lease: `tcp` requires a successful loopback connection, while `http` sends a fixed HTTP/1.1 request to `/` on the runtime-derived loopback endpoint and requires a final 2xx status-class response;
- never accepts a caller-supplied readiness URL, origin, host, or arbitrary path in the P0 contract;
- removes container and network on explicit termination and on partial-launch/readiness failures;
- returns an attested versioned lease and cleanup receipt.

The application request has no fields for privileged mode, host namespaces, devices, arbitrary mounts, environment variables, runtime sockets, external network enablement, or caller-controlled health destinations. Consumer-specific authentication/bootstrap remains consumer-owned and occurs only after the runtime has returned an attested ready lease.

## Alternatives

- **Consumer-owned Podman calls:** rejected because isolation policy would be duplicated and domain code would depend on infrastructure.
- **TCP reachability for HTTP readiness:** rejected because a listening socket can exist before the declared HTTP service is usable and can issue a lease that immediately fails consumer bootstrap.
- **Caller-supplied health URL/path:** rejected for P0 because it expands the runtime into an outbound-request authority and mixes consumer application semantics into the isolation contract.
- **Consumer login as runtime readiness:** rejected because login credentials and application authorization belong to the consumer, not the reusable isolation runtime.
- **Docker socket sidecar:** rejected because the socket is a high-authority control channel.
- **gVisor first:** deferred. gVisor is a planned stronger OCI backend but P0 first establishes the consumer-neutral contract and rootless lifecycle with broadly available Podman.
- **Kubernetes first:** deferred because a cluster is not required for standalone/local operation.

## Verification rule

Command-plan and fake-Podman process tests are necessary but not sufficient. Protocol readiness tests must prove that TCP-only acceptance does not satisfy an HTTP declaration, that a bounded 2xx response does, and that non-2xx/stalled responses fail closed. Release claims about container isolation require a real rootless Podman E2E lane that checks effective filesystem, namespace, capability, resource, network, and cleanup behavior. gVisor/containerd/Kubernetes adapters require their own parity and security evidence.
