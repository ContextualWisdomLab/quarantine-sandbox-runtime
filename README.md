# Quarantine Sandbox Runtime

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/ContextualWisdomLab/quarantine-sandbox-runtime)

`quarantine-sandbox-runtime` is the ContextualWisdomLab Rust runtime for **reusable hostile-workload isolation plus artifact-analysis evidence**.

The repository now supports two explicit consumer verticals:

- **Artifact analysis:** Wardnet and other security consumers submit hostile bytes and receive deterministic evidence. The consumer owns maliciousness verdicts, incidents, quarantine, blocking, notification, and retention.
- **Isolated application service:** Chat/Agent control planes such as `contextual-orchestrator` may request one explicitly approved application image to run as a short-lived service. The consumer owns conversation, agent/task/tool policy, authorization, image selection, secrets, and user-visible actions; this runtime owns isolation, lifecycle, resource limits, readiness, cleanup, and attestation.

The Core bounded context is `sandbox_execution`. `artifact_analysis` and `application_service` are separate Supporting bounded contexts. Podman/gVisor/containerd/Kubernetes details belong to infrastructure adapters rather than the domain model.

## Current active-PR capabilities

### Artifact analysis

- immutable SHA-256 artifact identity;
- bounded in-memory ingestion;
- deterministic PE, ELF, Mach-O, ZIP, PDF, OLE, script, text, and unknown-format classification;
- versioned request/evidence contracts;
- pluggable static analyzers and attributable failures;
- fail-closed `inconclusive` behavior when unavailable dynamic analysis is requested;
- explicit proof that the static foundation did not execute artifact content, use credentials, or make external network requests.

### Isolated application service

- immutable OCI image references only: `...@sha256:<64 lowercase hex>`;
- rootless Podman as the first infrastructure adapter;
- `--pull=never` so task launch does not become registry egress;
- per-sandbox `--internal --disable-dns` network;
- loopback-only random service publication;
- read-only root filesystem with one bounded `/tmp` tmpfs;
- all Linux capabilities dropped and `no-new-privileges`;
- isolated user/PID/IPC/UTS/cgroup namespaces and numeric non-root UID/GID;
- explicit CPU, RAM, PID, tmpfs, lease, readiness, and shutdown limits;
- no host devices, runtime sockets, broad host mounts, privileged mode, or ambient consumer credentials in the P0 contract;
- direct process invocation without a shell;
- bounded readiness gating and cleanup receipts;
- versioned service leases with isolation attestation.

**Important:** fake-Podman process-boundary tests verify command/lifecycle integration. They are not evidence that real Podman isolation has passed until the real-container E2E lane succeeds.

## Context Map

```text
                       +----------------------+
                       |   sandbox_execution  |
                       |        Core          |
                       | policy / resources   |
                       | lifecycle / lease    |
                       | endpoint / cleanup   |
                       | attestation          |
                       +----------+-----------+
                                  ^
                  consumes         |          consumes
                  isolation        |          isolation
          +-----------------------+ +------------------------+
          |                                                  |
+---------+----------+                              +--------+-----------+
| artifact_analysis  |                              | application_service|
| Supporting context |                              | Supporting context |
| bytes / evidence   |                              | approved app intent|
+---------+----------+                              +--------+-----------+
          |                                                  |
          v                                                  v
       Wardnet                                   contextual-orchestrator
 verdict / incident /                           chat / agent / task /
 quarantine / response                          authorization / secrets
```

Consumer repositories integrate through versioned contracts or an Anti-Corruption Layer. They do not copy this source or directly depend on Podman/containerd implementation types.

## Security defaults

The P0 application-service profile is deliberately restrictive:

- no implicit image pull;
- no arbitrary Internet egress;
- no credential injection;
- no host network/PID/IPC;
- no privileged mode or host devices;
- no Docker/Podman/containerd socket;
- no arbitrary host-path mount;
- no 0.0.0.0/`::` publication;
- no shell interpretation of application argv.

A later controlled-egress or secret-broker profile must be a separate accepted contract; it may not weaken these defaults implicitly.

## Development

```bash
cargo fmt --check
cargo test --locked --workspace --all-targets
cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
python3 scripts/validate_repository.py
```

Canonical implementation documents:

- `docs/PRD.md`
- `docs/TRD.md`
- `docs/ARCHITECTURE.md`
- `docs/SECURITY.md`
- `docs/THREAT_MODEL.md`
- `docs/TEST_STRATEGY.md`
- `docs/OPERABILITY.md`
- `docs/product-technical-gap-baseline.md`
- `docs/superpowers/specs/2026-09-01-isolated-application-service-design.md`
- `docs/superpowers/plans/2026-09-01-isolated-application-service.md`
- `docs/adr/README.md`
- `docs/doctoring/STANDARD_TRACEABILITY.md`

The repository name predates the broader Chat/Agent consumer boundary. A less security-specific repository name is tracked as a pre-GA naming gap rather than silently changing consumer coordinates without an authorized repository-settings migration path.
