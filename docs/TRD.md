# Technical Requirements Document

## Runtime stack

- Rust 1.97.x baseline, Edition 2024.
- `unsafe` forbidden in this crate by default.
- `serde`/JSON Schema Draft 2020-12 for versioned wire contracts.
- SHA-256 for immutable artifact and effective-policy identity; fresh operating-system entropy for application-service invocation/resource identity.
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
│   └── mod.rs
├── infrastructure/           Runtime/process adapters
│   ├── bounded_command.rs
│   ├── mod.rs
│   └── podman.rs
└── lib.rs                    Public facade only
```

Podman and bounded subprocess execution are infrastructure adapters, not domain objects. Core and Supporting contexts expose backend-neutral contracts; additional backends belong under the infrastructure boundary without changing consumer request/lease semantics merely because the runtime technology changes.

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

Schema `1.1.0` records:

- schema version;
- original request ID;
- image digest reference;
- runtime backend ID;
- sandbox/network IDs;
- policy ID and canonical SHA-256 of every effective policy field;
- loopback endpoint;
- start/expiry/shutdown values;
- P0 isolation attestation.

The serialized lease is evidence/correlation, not destructive authority. A runtime-issued in-process lease additionally carries crate-private, non-serializable cleanup authority. Deserializing the public lease shape cannot recreate that authority; explicit termination fails closed as `CleanupAuthorityUnavailable` when it is absent.

### `CleanupReceipt`

Successful receipt exists only when both container and network removal succeeded.

## Rootless Podman adapter

### Planning

`RootlessPodmanAdapter::plan_at` validates request/policy, obtains 128 bits of operating-system entropy, encodes it as 32 lowercase hexadecimal characters, and binds that invocation identity across the generated `qsr-app-*` container name, `qsr-net-*` network name, and runtime identity label. Entropy failure returns `RuntimeIdentityUnavailable`; there is no deterministic fallback. Raw caller strings and `request_id` remain correlation/idempotency data and are not infrastructure cleanup authority.

Draft #21 owns this invocation-collision repair. Draft #40 remains a separate post-create ownership contract: once Podman returns an admitted concrete container ID, lifecycle/destructive operations must bind to that acquired ID rather than assuming the generated correlation name remains authoritative.

### Backend verification

Before creating isolation resources:

```text
podman info --format {{.Host.Security.Rootless}}
```

must produce `true` after trimming. Backend security information is then inspected and required P0 seccomp/LSM host capability must be present; host capability by itself does not prove per-sandbox confinement.

### Network

Each service creates a runtime-named internal network using:

```text
podman network create --internal --disable-dns <network>
```

The current #127 candidate immediately inspects that exact generated `qsr-net-*` name after creation, requires exactly one inspection object with a canonical lowercase 64-hex Podman network `id`, and rewrites the container-create `--network` selector to that acquired ID. Missing or malformed identity fails closed; there is no generated-name fallback for container binding. The generated name remains the consumer-visible correlation and the network-object inspection key.

This first acquired-identity stage does not yet prove effective egress isolation. Current production still does not verify the exact acquired container's `NetworkSettings.Networks` membership, still derives private network cleanup authority from the public correlation value, and still removes networks with `--force`. Executed predecessor #124 exact `6c653d36f16b18af353949e0f0152c8527f48396` proved the resulting false-GREEN: different, missing, and additional effective attachments all reached lease publication instead of `IsolationVerificationFailed { control_name: "sandbox_network_binding" }`.

Canonical successor #127 therefore remains a Draft candidate. The already-implemented first stage is: create the invocation-local `qsr-net-*` name → inspect that exact name before container creation → acquire the full Podman network `.ID` → bind container `--network` to the acquired ID. The remaining acceptance sequence is: inspect the exact acquired container → require exactly one effective `NetworkSettings.Networks` membership whose `NetworkID` equals the acquired identity → keep the generated name as public lease correlation → retain the acquired ID independently as private cleanup authority → remove the owned network without network-level `--force`. Foreign membership must make cleanup fail closed rather than delete another member. None of those remaining stages is GREEN until it is implemented and passes exact-head gates.

### Container creation

Required controls include:

```text
--pull=never
--read-only
--read-only-tmpfs=false
--http-proxy=false
--image-volume=ignore
--no-hosts
--systemd=false
--sdnotify=ignore
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
--network <acquired network ID>
--publish 127.0.0.1::<container-port>/tcp
```

The immutable image reference is appended before application argv, so application arguments cannot be interpreted as Podman options. No shell is involved. Applied argv or inspect configuration is not equivalent to live resource-enforcement evidence; Draft #19 carries the P0 resource-attestation RED for tmpfs/wall-time and subsequent live cgroup/mount/termination proof.

### Start, effective verification, readiness, and lease

The current #127 candidate sequence is:

1. Create the internal network under the invocation-local `qsr-net-*` name.
2. Inspect that exact network name and admit only a canonical lowercase 64-hex Podman network ID.
3. Bind container creation to the acquired network ID, then create the container and admit only an exact 64-character ASCII hexadecimal container identifier from successful create output.
4. Start the container.
5. Inspect the runtime-owned network object and exact container identity/configuration.
6. Inspect process seccomp/capability/LSM evidence and fail closed unless every implemented P0 isolation control is positively verified.
7. Query the requested port mapping only after isolation verification succeeds.
8. Accept only a single IPv4 loopback `127.0.0.1:<nonzero-port>` mapping.
9. Poll bounded TCP readiness using operator timeout/poll policy.
10. Return a lease only after effective-isolation checks and readiness succeed.

The acquired-network-ID chronology is implemented on #127, but exact-container effective attachment verification is not. Until the candidate inspects the acquired container's effective network membership and rejects different, missing, or additional attachments before port/readiness, network-object policy plus an ID-bound create request remains configuration intent rather than positive attachment proof.

P0 HTTP readiness deliberately uses TCP reachability because no consumer-supplied health path is accepted yet. A future typed HTTP health contract may refine this without accepting arbitrary URLs.

### Cleanup

- Network-create failure: no resources assumed.
- Container-create failure: remove network.
- Start failure: remove container and network.
- Isolation/port/readiness failure: stop/remove container and remove network.
- Explicit termination: require runtime-owned non-serializable cleanup authority, stop with its captured shutdown grace, remove its container target, then remove its network target.
- Consumer-deserialized lease evidence without runtime cleanup authority: return `CleanupAuthorityUnavailable` before any destructive Podman command.
- If cleanup cannot be proven, return `CleanupFailed` rather than the original error as though cleanup succeeded.

Current production still derives private network cleanup authority from the public correlation value and uses forceful network removal. Draft #127 explicitly forbids treating that correlation as acquired destructive identity: the generated name remains public evidence, while the full inspected Podman network ID must be retained privately and used for non-force network removal. That repair is not considered implemented or GREEN until it exists in production source and passes the unchanged successor tests on one exact head.

Current #21 ancestry still has the independent #40 exact acquired-container-ID lifecycle RED checked in; generated-name versus acquired-ID selection must be resolved there before this cleanup path is considered ownership-complete. `--timeout` expresses an intended container lifetime bound, but configuration alone is not release-grade proof that wall-time termination occurred. Durable crash/restart orphan reclamation also requires the Recovery context/reaper before GA.

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