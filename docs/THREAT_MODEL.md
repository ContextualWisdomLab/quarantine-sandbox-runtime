# Threat Model

## Assets

- host kernel and filesystem;
- consumer credentials and provider secrets;
- Agent/chat task authority;
- security verdict/incident authority;
- immutable application/artifact identity;
- analysis evidence and runtime attestation;
- CPU, RAM, PID, storage and network capacity;
- container backend control plane;
- loopback service endpoint and lease identity.

## Adversaries

- malicious artifact author;
- malicious or compromised application image;
- prompt-injected Agent observation attempting to request broader runtime authority;
- compromised/buggy consumer submitting malformed or excessive requests;
- compromised analyzer/backend returning misleading output;
- local unprivileged attacker attempting to interfere with runtime resources;
- supply-chain attacker replacing dependency, action, or application image.

## Trust-boundary diagram

```mermaid
flowchart LR
    U[Untrusted artifact/app request] --> R[Runtime validation]
    C[Consumer authorization\nexternal authority] --> R
    R --> P[Isolation policy]
    P --> B[Rootless/strong sandbox backend]
    B --> W[Untrusted workload]
    W --> E[Bounded evidence/readiness]
    E --> R
    R --> O[Lease/evidence/cleanup receipt]
    O --> C

    S[Consumer/provider secrets] -. never P0 input .- R
    H[Host runtime socket/devices] -. never P0 mount .- W
```

## STRIDE-style threats and controls

### Spoofing

- **Mutable image tag presented as stable identity:** reject; require SHA-256 digest.
- **Backend claims rootless while running privileged:** P0 probes rootless; real E2E and later backend attestation must verify effective runtime state.
- **Analyzer reports another producer identity:** failure/evidence attribution binds configured producer identity; reported metadata remains untrusted.
- **Consumer reuses another task's lease:** request/lease IDs are opaque and consumers must bind them to their own authorized session/task; stale/expired lease rejection is a consumer ACL requirement.

### Tampering

- **Artifact bytes mutate during analysis:** immutable in-process bytes plus SHA-256 identity.
- **Application image changes between authorization and launch:** immutable digest plus `--pull=never`.
- **Sandbox request raises limits:** operator `IsolationPolicy` is authoritative; consumer request is bounded below it.
- **Infrastructure name injection:** container/network names are derived from SHA-256, not raw request text.
- **Shell injection through command args:** direct `Command` argv; no shell.

### Repudiation

- **Consumer disputes which image/policy ran:** versioned lease records request, image, backend, policy, endpoint, start/expiry and isolation facts.
- **Cleanup is assumed without evidence:** success requires explicit cleanup receipt; failures are not rewritten as success.
- **Analysis provenance lost:** deterministic evidence IDs/order and producer attribution.

### Information disclosure

- **Secrets leak into workload:** no P0 secret/env/mount capability.
- **Host files leak:** read-only image plus bounded tmpfs; no arbitrary host-path mount.
- **Runtime socket leaks host control:** no Docker/Podman/containerd socket request or mount.
- **Backend diagnostics echo hostile data:** errors use stable codes; future captured output must remain bounded and sanitized.
- **Service becomes externally reachable:** publication is only `127.0.0.1` and returned mapping is validated.

### Denial of service

- **CPU/RAM/PID exhaustion:** explicit limits within operator maxima.
- **Unbounded temporary storage:** bounded tmpfs.
- **Long-running application:** lease/container timeout plus consumer termination; durable orphan reaper is a GA gap.
- **Readiness hangs:** bounded timeout/poll interval.
- **Excessive artifacts/metadata:** ingestion/context limits.
- **Container/backend process storm:** one sandbox per accepted request and PID/resource policy; concurrency/admission control is a future operability slice.

### Elevation of privilege

- **Privileged container/capability escalation:** no privileged request surface, all capabilities dropped, no-new-privileges.
- **Host namespace access:** isolated user/PID/IPC/UTS/cgroup namespaces and internal network.
- **Device/GPU access:** no P0 device passthrough.
- **Container escape through kernel vulnerability:** rootless containment reduces impact but does not remove shared-kernel risk; stronger gVisor/VM adapter is required for higher-risk profiles.
- **Agent prompt injection asks sandbox for secrets/network:** Chat/Agent consumer authorization is separate, and P0 contract has no such fields.

## Key abuse cases

### Application tries Internet exfiltration

Expected result: no externally routed P0 network path. Real-container E2E must verify this. A controlled-egress profile is a separate future contract and cannot be enabled by application input.

### Application tries to modify image filesystem

Expected result: read-only rootfs denial; only explicit bounded `/tmp` is writable. E2E must verify `/tmp` is noexec and rootfs writes fail.

### Application forks indefinitely

Expected result: PID and CPU/memory limits constrain the workload; inability to enforce a requested limit fails closed.

### Consumer passes `--privileged` as application argument

Because application args are appended **after** image identity, they are application argv and cannot become Podman flags. Consumer contract does not expose raw Podman option injection.

### Runtime crashes after launch

Podman timeout bounds application process lifetime, but stale container/network cleanup is not fully guaranteed. Durable lease/orphan reclamation is a release-blocking gap before GA.

## Residual risks

- shared-kernel container escape vulnerabilities;
- host-specific rootless/cgroup/network behavior;
- supply-chain trust of an immutable but malicious image;
- loopback service vulnerability exploitable by another local process;
- process crash before cleanup producing stale isolation objects;
- no authenticated IPC/API layer in the current library foundation;
- no controlled secret capability for applications that legitimately require credentials.

These risks must be addressed by stronger adapters, admission policy, local identity/IPC controls, durable reaping, and explicit capability brokers rather than weakening the P0 sandbox.
