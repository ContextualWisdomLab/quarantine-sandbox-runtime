# Technical Requirements Document

## Runtime stack

- Rust 1.97.x baseline, Edition 2024.
- `unsafe` forbidden in this crate by default.
- `serde`/JSON Schema Draft 2020-12 for versioned wire contracts.
- SHA-256 for immutable artifact identity and deterministic infrastructure-safe sandbox names.
- Rootless Podman as the first application-service infrastructure adapter.
- Future adapters may target OCI/containerd/gVisor and Kubernetes RuntimeClass without changing consumer domain authority.

## Module architecture

```text
src/
├── artifact_analysis/         Supporting bounded context
│   ├── contracts.rs
│   ├── ingestion.rs
│   └── runtime.rs
├── application_service/      Supporting bounded context
│   └── mod.rs
├── sandbox_execution/        Core bounded context
│   ├── mod.rs
│   └── podman.rs             Infrastructure adapter, temporary colocated module
└── lib.rs                    Public facade only
```

The Podman implementation is a Core infrastructure adapter, not a domain object. If additional backends are added, extract a backend port/adapter package structure without changing the consumer request/lease semantics unnecessarily.

## Application-service contracts

### `IsolationPolicy`

Operator-owned maxima:

- memory bytes;
- CPU millicores;
- process count;
- lease seconds;
- tmpfs bytes;
- readiness timeout/poll interval;
- shutdown grace;
- numeric non-root UID/GID;
- policy identifier.

The consumer cannot request more than these values or mutate isolation capabilities.

### `ApplicationServiceRequest`

- schema `1.0.0`;
- opaque bounded request ID;
- immutable `@sha256` OCI image reference;
- TCP container service port;
- protocol metadata (`tcp`/`http`);
- bounded direct argv with no shell;
- bounded resource request.

There are intentionally no environment, mount, device, privileged, host-namespace, runtime-socket, public-bind, or network-enable fields.

### `ApplicationServiceLease`

Records:

- schema version;
- original request ID;
- image digest reference;
- runtime backend ID;
- sandbox/network IDs;
- policy ID;
- loopback endpoint;
- start/expiry/shutdown values;
- P0 isolation attestation.

### `CleanupReceipt`

Successful receipt exists only when both container and network removal succeeded.

## Rootless Podman adapter

### Planning

`RootlessPodmanAdapter::plan_at` validates request/policy and produces deterministic command argv. Sandbox/network names are derived from SHA-256 over request ID, immutable image, policy ID, and start time; raw caller strings are not used as container/network names.

### Backend verification

Before creating isolation resources:

```text
podman info --format {{.Host.Security.Rootless}}
```

must produce `true` after trimming. Any other value fails closed.

### Network

Each service creates a unique bridge network using:

```text
podman network create --internal --disable-dns --ignore <network>
```

P0 does not add another network. This is a default-deny egress profile rather than a general network sandbox API.

### Container creation

Required controls include:

```text
--pull=never
--read-only
--read-only-tmpfs=false
--cap-drop=all
--security-opt=no-new-privileges
--userns=auto
--ipc=none
--pid=private
--uts=private
--cgroupns=private
--restart=no
--log-driver=none
--timeout <lease_seconds>
--user <uid>:<gid>
--pids-limit <n>
--memory <bytes>
--cpus <decimal cores>
--tmpfs /tmp:rw,noexec,nosuid,nodev,size=<bytes>
--network <internal network>
--publish 127.0.0.1::<container-port>/tcp
```

The immutable image reference is appended before application argv, so application arguments cannot be interpreted as Podman options. No shell is involved.

### Start, readiness, and lease

1. Create network.
2. Create container.
3. Start container.
4. Query the requested port mapping.
5. Accept only a single IPv4 loopback `127.0.0.1:<nonzero-port>` mapping.
6. Poll bounded TCP readiness using operator timeout/poll policy.
7. Return a lease only after readiness.

P0 HTTP readiness deliberately uses TCP reachability because no consumer-supplied health path is accepted yet. A future typed HTTP health contract may refine this without accepting arbitrary URLs.

### Cleanup

- Network-create failure: no resources assumed.
- Container-create failure: remove network.
- Start failure: remove container and network.
- Port/readiness failure: stop/remove container and remove network.
- Explicit termination: stop with lease shutdown grace, remove container, remove network.
- If cleanup cannot be proven, return `CleanupFailed` rather than the original error as though cleanup succeeded.

The container timeout limits application process lifetime even if the runtime process disappears, but stale container/network object reclamation still requires a durable reaper before GA.

## Artifact-analysis technical contract

Existing artifact-analysis code remains source-compatible through root crate re-exports after moving implementation under `artifact_analysis`.

- Ingestion validates size/name before cloning bytes.
- SHA-256 binds artifact identity.
- Format recognition is non-executing.
- Static analyzers implement `StaticAnalyzer` and emit normalized findings/failures.
- Evidence identifiers and ordering are deterministic for the same request/configuration/bytes.
- Dynamic profiles without a worker fail closed as incomplete rather than silently downgrading to static completeness.

## Backend evolution

### gVisor/containerd

A future adapter may use OCI `runsc`/containerd. It must provide semantically comparable lease/cleanup/isolation evidence and document any stronger/weaker behavior. Consumer domain types must not change merely because the backend changes.

### Kubernetes

A future managed deployment uses a runtime-selected workload object (for example Pod/Job plus Service or port-forward mechanism) and an explicit RuntimeClass where applicable. It must retain task-scoped lifetime, non-root/read-only/resource/network/secret rules and not expose an externally routable service by default.

## Performance and resource behavior

This runtime prioritizes containment correctness over raw startup latency. Measurements must report actual backend/hardware. No performance claim is accepted from command-plan tests.

Required metrics for a real backend include:

- launch-to-ready latency;
- cleanup latency;
- peak runtime-controller RSS;
- application memory/CPU/PID enforcement evidence;
- orphan resource count after failure/restart;
- network reachability result;
- concurrent sandbox capacity under bounded policy.

## Compatibility

- JSON contract versions are explicit.
- Public Rust facade should preserve existing artifact-analysis names when moving internal paths.
- New incompatible contract behavior requires a version increment and compatibility tests.
- No silent coercion of mutable image tags, unsupported profiles, malformed backend output, or excessive resource requests.
