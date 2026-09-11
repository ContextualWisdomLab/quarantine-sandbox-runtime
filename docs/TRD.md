# Technical Requirements Document

## Runtime stack

- Rust 1.97.x baseline, Edition 2024.
- `unsafe` forbidden in this crate by default.
- `serde`/JSON Schema Draft 2020-12 for versioned wire contracts.
- SHA-256 for immutable artifact/source/gate identity and infrastructure-safe correlation names.
- Rootless Podman as the first application-service and one-shot command infrastructure adapter.
- Future adapters may target OCI/containerd/gVisor and Kubernetes RuntimeClass without changing consumer domain authority.

## Module architecture

```text
src/
├── artifact_analysis/                 Supporting bounded context
│   ├── contracts.rs
│   ├── ingestion.rs
│   └── runtime.rs
├── application_service/              Supporting bounded context
│   ├── command_execution.rs
│   ├── coordinator.rs
│   └── mod.rs
├── sandbox_execution/                Core bounded context
│   └── mod.rs
├── infrastructure/                   Runtime/process adapters
│   ├── application_service_backend.rs
│   ├── bounded_command.rs
│   ├── podman.rs
│   ├── podman_runtime_gate_binding.rs
│   ├── runtime_gate_artifact.rs
│   └── mod.rs
├── bin/
│   └── qsr_runtime_gate.rs            Runtime-owned pre-exec hold/release process
├── pr_source_artifact.rs             Exact-revision host staging boundary
├── main.rs                           CLI transport
└── lib.rs                            Public facade only
```

Podman, runtime-gate composition, and bounded subprocess execution are infrastructure concerns, not consumer/domain objects. Core and Supporting contexts expose backend-neutral contracts; additional backends belong under the infrastructure boundary without changing consumer request/result/lease semantics merely because runtime technology changes.

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

### `CleanupReceipt`

Successful receipt exists only when both container and network removal succeeded.

## Rootless Podman application-service adapter

### Planning

`RootlessPodmanAdapter::plan_at` validates request/policy and produces deterministic command argv. Service sandbox/network names are derived from SHA-256 over request ID, immutable image, policy ID, and start time; raw caller strings are not used directly as container/network names. Separate application-service lifecycle-ownership work must preserve exact backend identity once a container has been created; a generated correlation name is not sufficient destructive authority after that point.

### Backend verification

Before creating isolation resources:

```text
podman info --format json
```

must report rootless operation and the required host security capabilities. Host capability by itself does not prove per-sandbox confinement.

### Network

Each service creates a runtime-named internal network using:

```text
podman network create --internal --disable-dns <network>
```

The network object is inspected for the internal/DNS-disabled policy. P0 does not intentionally add another network. Effective attachment and negative-egress claims require runtime evidence; configured network state alone is not a kernel-enforcement claim.

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
--network <internal network>
--publish 127.0.0.1::<container-port>/tcp
```

The immutable image reference is separated from Podman options with `--` before application argv. No shell is involved. Applied argv or inspect configuration is not equivalent to live resource-enforcement evidence; CPU/RAM/PID, tmpfs, wall-time, network, and LSM claims require the corresponding real-runtime evidence.

### Start, effective verification, readiness, and lease

1. Create the internal network.
2. Create the container and acquire its backend identity.
3. Start the container.
4. Inspect the runtime-owned network and exact container identity/configuration.
5. Inspect process seccomp/capability/LSM evidence and fail closed unless every implemented P0 isolation control is positively verified.
6. Query the requested port mapping only after isolation verification succeeds.
7. Accept only a single IPv4 loopback `127.0.0.1:<nonzero-port>` mapping.
8. Poll bounded readiness under operator timeout/poll policy.
9. Return a lease only after effective-isolation checks and readiness succeed.

### Cleanup

- Network-create failure: no resources assumed.
- Container-create failure: remove only runtime-owned network state that can be proven safe to remove.
- Start failure: remove the created container and network through their owned identities.
- Isolation/port/readiness failure: stop/remove container and remove network.
- Explicit termination: stop with lease shutdown grace, remove container, remove network.
- If cleanup cannot be proven, return `CleanupFailed` rather than the original error as though cleanup succeeded.

`--timeout` expresses intended container lifetime configuration, not release-grade proof that wall-time termination occurred. Durable crash/restart orphan reclamation also requires the Recovery context/reaper before GA.

## One-shot command execution

### `CommandExecutionRequest`

The command contract is consumer-neutral and bounded. It carries an opaque request/correlation identifier, immutable digest-pinned OCI image, direct argv, the same policy-bounded CPU/RAM/PID/tmpfs/lease resource request, and an optional exact-revision source artifact. It exposes no shell, ambient environment map, secret injection, device, runtime socket, privileged mode, host namespace, public bind, or arbitrary egress authority.

`request_id` is correlation metadata, not the unique backend-resource identity for a one-shot execution. Production command resource identity includes runtime-generated entropy so two otherwise identical same-second requests cannot alias a container name. After successful create, the exact ID written by Podman's runtime-owned `--cidfile` is the destructive lifecycle authority.

### Optional exact-revision source staging

`PrSourceArtifactInput` requires an absolute host directory, canonical lowercase Git object identity (SHA-1 or SHA-256 width), and the caller's canonical SHA-256 tree digest. Staging:

- rejects a caller-supplied root that is not itself a directory under no-follow metadata;
- preserves exact Unix pathname bytes, including literal backslashes and non-UTF-8 names;
- rejects symlinks/devices/sockets and other unsupported descendants;
- bounds regular-file count and total bytes;
- re-hashes pathname length/path bytes/content length/content bytes in sorted byte order;
- strips executable bits from staged regular files;
- uses an owner-only host staging root and exposes only the sanitized tree to the command path;
- binds the staged tree at `/workspace` as `ro,noexec,nosuid,nodev` when requested.

The current root-object repair proves no-follow type plus device/inode continuity across initial canonicalization. It does not claim the later pathname-based recursive traversal is fully race-free against concurrent same-principal mutation; that stronger filesystem-capability boundary remains separate hardening work.

### Immutable runtime gate

`RuntimeGateArtifact` verifies a host-owned gate before it can become container execution authority:

- expected SHA-256 is exact lowercase 64-hex;
- declared architecture must equal the runtime host architecture;
- source must be a no-follow regular file;
- bytes must match the expected digest;
- ELF admission is limited to self-contained ELF64 `ET_EXEC`/`ET_DYN` with a bounded program-header table, at least one `PT_LOAD`, and no `PT_INTERP` dependency on workload-image code;
- executable `e_machine` must match the declared/host architecture;
- verified bytes are copied to a private read-only/executable staging path.

A runtime-gate package or digest check is prerequisite identity evidence; it is not itself effective container isolation evidence.

### Production `RuntimeGatePodmanAdapter` lifecycle

The release path does not invoke the legacy direct-consumer container lifecycle. Production composition is:

```text
verify/stage immutable gate
→ stage optional exact-revision source
→ verify rootless/security backend capability
→ create container with gate as OCI PID 1 and consumer argv held behind a one-time token
→ acquire exact container ID from runtime-owned cidfile
→ verify configured image/resource/tmpfs/namespace/network-none/mount contradictions
→ start gate only
→ verify live effective seccomp/capability/LSM evidence
→ authorize bounded one-time release against the exact container ID
→ require trusted QSR_GATE_RELEASED acknowledgement
→ detach the release stdin/control channel
→ exec exact consumer argv
→ bounded wait / timeout termination / log retrieval
→ exact-ID cleanup
```

Command containers use `--network none`; they create no per-command network object and publish no port. The configured state includes read-only root, capability drop, no-new-privileges, isolated user/PID/IPC/UTS/cgroup namespaces, non-root UID/GID, CPU/RAM/PID bounds, exact nonzero lease timeout, and one exact hardened `/tmp` tmpfs. Output is retrieved after exit from a finite `k8s-file` log path; command wait and log collection remain bounded operations.

A failure before successful create has no container identity to destroy. After successful create, cleanup is scoped to the acquired exact backend ID; malformed stdout cannot promote the generated `qsr-cmd-*` correlation name into destructive authority. Timeout handling requires a successful kill before accepting post-kill wait evidence. Cleanup failure remains explicit evidence and is not silently hidden behind the preceding failure.

`RootlessPodmanAdapter::run_legacy_command_at_for_test` exists only under debug assertions so historical/focused process tests can exercise the lower-level path. Release consumers use `RuntimeGatePodmanAdapter`.

### Remaining release evidence

Configured `container inspect` state is not proof of kernel enforcement. The command profile is not release-ready until the same exact candidate demonstrates, through real rootless execution, effective cgroup-v2 CPU/RAM/PID bounds, actual `/tmp` mount options/size, behavioral wall-time kill/cleanup, negative egress, and a positive effective LSM. The dedicated positive-LSM lane is authoritative only when an eligible SELinux runner actually executes it.

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

A future adapter may use OCI `runsc`/containerd. It must provide semantically comparable result/lease/cleanup/isolation evidence and document any stronger/weaker behavior. Consumer domain types must not change merely because the backend changes.

### Kubernetes

A future managed deployment uses a runtime-selected workload object (for example Pod/Job plus Service or port-forward mechanism) and an explicit RuntimeClass where applicable. It must retain task-scoped lifetime, non-root/read-only/resource/network/secret rules and not expose an externally routable service by default.

## Performance and resource behavior

This runtime prioritizes containment correctness over raw startup latency. Measurements must report actual backend/hardware. No performance claim is accepted from command-plan or fake-Podman tests.

Required metrics for a real backend include:

- launch-to-ready latency for services and create-to-result latency for commands;
- cleanup latency;
- peak runtime-controller RSS;
- workload memory/CPU/PID enforcement evidence;
- actual tmpfs mount/options/size evidence;
- behavioral lease/wall-time enforcement;
- orphan resource count after failure/restart;
- network reachability/negative-egress result;
- concurrent sandbox capacity under bounded policy.

## Compatibility and release evidence

- JSON contract versions are explicit.
- Public Rust facade preserves existing artifact-analysis names when internal paths move.
- New incompatible contract behavior requires a version increment and compatibility tests.
- No silent coercion of mutable image tags, unsupported profiles, malformed backend output, or excessive resource requests.
- Every merge/release candidate requires exact-head fmt/tests/Clippy/rustdoc, 100% owned-production statement/function/region/branch coverage, required real-runtime security evidence, review/security/dependency/SBOM gates, immutable package/provenance, reproducibility, and rollback evidence.
- Predecessor GREEN results do not transfer after source or documentation changes.
