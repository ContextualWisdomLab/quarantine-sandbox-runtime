# Security

## Security objective

The runtime is a privilege-reduction boundary for workloads that must not inherit the caller's ambient authority. It is not a guarantee that containers are equivalent to virtual machines, and it does not convert untrusted code into trusted code.

## Trust zones

### Trusted operator inputs

- installed runtime binary and locked dependencies;
- operator-selected backend executable;
- `IsolationPolicy` maxima and non-root runtime identity;
- released application image digests admitted through a separate trusted supply-chain process.

These inputs are trusted for policy but still validated for structural correctness.

### Untrusted consumer/workload inputs

- artifact bytes and names;
- application image reference text;
- request IDs;
- container service port/protocol;
- application argv;
- requested resources;
- analyzer/backend output;
- service readiness behavior;
- application-generated network/file/process behavior.

## P0 application-service controls

### Image authority

- Require immutable `@sha256:<64 lower-case hex>` reference.
- Use `--pull=never` during task launch.
- Reject mutable tag-only identity.
- Image signature/vulnerability/admission policy is a separate supply-chain gate; a digest alone proves identity, not trustworthiness.

### Process privileges

- rootless Podman required;
- `--cap-drop=all`;
- `--security-opt=no-new-privileges`;
- automatic user namespace;
- numeric non-root UID/GID;
- private PID/IPC/UTS/cgroup namespaces;
- no privileged flag and no host namespace request surface.

### Filesystem and devices

- read-only root filesystem;
- disable implicit writable tmpfs created by read-only mode;
- create only bounded `/tmp` tmpfs with `noexec,nosuid,nodev`;
- no arbitrary host-path mount request in P0;
- no device passthrough;
- no Docker/Podman/containerd socket;
- no SSH agent or host keychain mount.

### Network

- one per-sandbox Podman bridge with `--internal --disable-dns`;
- no second/external network in P0;
- publish exactly one requested TCP service on host `127.0.0.1` with random host port;
- reject malformed/non-loopback port mapping;
- no wildcard/external listener publication.

This is a strong default-deny posture for the P0 profile, but real-container E2E must prove effective external reachability behavior before release claims.

### Credentials

P0 supplies no consumer/provider credentials, cloud tokens, API keys, cookies, agent secrets, or arbitrary environment map. Sensitive operations remain in the consumer or a future purpose-limited capability broker.

### Resources and lifetime

- operator policy bounds all requests;
- CPU, RAM, PID, tmpfs and lease limits are explicit;
- Podman container timeout mirrors lease duration;
- readiness timeout/polling is bounded;
- cleanup is required after partial launch and explicit termination;
- cleanup uncertainty is a failure.

Rootless cgroup/resource support depends on host configuration. Requested enforcement that the backend cannot provide must fail rather than silently proceed.

## Artifact-analysis controls

- Preserve original artifact bytes and SHA-256 identity.
- Bound file/name/context size before cloning/processing.
- Never execute artifact content in the control process.
- Static analyzer evidence must not be labeled runtime behavior.
- Analyzer failure remains attributable to the configured analyzer identity.
- No production credential is available to analysis workers.
- Current static foundation performs no outbound network access.
- Future detonation profiles require stronger worker/sandbox evidence and retain the same consumer verdict boundary.

## Secrets and logging

Do not log:

- artifact bytes;
- application argv when it may contain user data unless a future approved redaction/retention contract exists;
- credential values;
- provider tokens/cookies;
- unbounded backend stdout/stderr;
- raw user prompt/message bodies.

Stable operation/error codes are preferred to echoing hostile payload text.

## Supply chain

- `Cargo.lock` is committed and validated.
- GitHub Actions are pinned by full commit SHA.
- Release requires dependency/advisory review, SBOM, provenance, and reproducible artifact evidence according to repository policy.
- OCI image digest pinning is necessary but not sufficient; production admission should add signature/vulnerability/provenance policy before accepting third-party images.

## Stronger isolation

Rootless application containers reduce ambient authority but still share a kernel. A gVisor `runsc`, Kata/VM, Firecracker, or platform-specific VM profile may be appropriate for hostile detonation or stronger tenant separation. Those are separate adapters/profiles with their own compatibility and performance evidence, not implicit upgrades to the P0 attestation.

## Compliance posture

The architecture is designed to produce evidence useful for SOC 2/CSAP-style control readiness, but this repository does not claim certification. NIST SP 800-190 is treated as authoritative container-security guidance; OCI Runtime Specification is the interoperability baseline for runtime semantics.
