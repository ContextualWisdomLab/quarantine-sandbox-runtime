# Test Strategy

## Test principles

- Behavior changes are test-first.
- Tests must cross the real production boundary appropriate to the claim.
- Command-plan tests prove deterministic command construction, not container isolation.
- Fake-backend tests prove process orchestration/cleanup semantics, not the effective kernel/network boundary.
- Real Podman/gVisor/VM E2E is required before claiming that backend's isolation behavior.
- Pending/skipped/absent/failed/stale evidence is non-passing.

## Unit and contract tests

### Artifact analysis

- request/schema validation;
- bounded source context;
- artifact name/size limits;
- SHA-256 identity;
- PE/ELF/Mach-O/ZIP/PDF/OLE/script/text/unknown classification;
- analyzer ordering/failure attribution;
- deterministic evidence IDs/order;
- dynamic-profile unavailable => `inconclusive`;
- evidence bundle and runtime boundary validation.

### Application service

- schema version rejection;
- empty/oversized/control-character request ID rejection;
- mutable/tag-only/whitespace/control/uppercase/malformed digest rejection;
- port zero rejection;
- command count/size/control-character rejection;
- every operator-policy zero/inconsistent field rejection;
- every resource zero and every resource-over-policy rejection;
- lease expiry overflow rejection;
- stable protocol/backend codes;
- deterministic sandbox/network names;
- required Podman isolation flags;
- absence of privileged, host-network, runtime-socket, device, arbitrary mount, environment, and external-bind request paths.

## Process-boundary tests

On Linux, a controlled fake Podman executable must exercise the actual `std::process::Command` path without a shell.

Required cases:

- executable missing => `BackendInvocationFailed`;
- rootless probe nonzero => `BackendCommandFailed`;
- rootless probe returns false => `BackendNotRootless`;
- network create failure;
- container create failure with network cleanup;
- container start failure with container/network cleanup;
- port query failure with stop/remove/network cleanup;
- malformed/non-loopback/zero port mapping => fail closed and cleanup;
- successful launch reaches a real local `TcpListener`, returns versioned lease/attestation, and explicit terminate returns cleanup receipt;
- readiness timeout removes resources;
- cleanup command failure => `CleanupFailed` after all cleanup attempts.

Fake backend logs are test fixtures only and must not become production diagnostics.

## Real rootless Podman E2E

Required before release-ready application isolation claim:

1. Prove Podman is rootless on the runner/host.
2. Load or build a tiny pinned test image locally; launch with `--pull=never`.
3. Verify service is reachable only through returned `127.0.0.1` port.
4. Verify root filesystem write fails.
5. Verify `/tmp` write succeeds within limit and execute from `/tmp` is denied.
6. Verify effective capabilities are empty and no-new-privileges is set.
7. Verify workload UID/GID is non-root as configured.
8. Verify PID/IPC/UTS/cgroup namespace posture expected by the profile.
9. Verify external route/DNS are absent from the application network.
10. Verify CPU/RAM/PID controls are visible/enforced on the host profile used.
11. Verify application cannot see runtime sockets, host devices, arbitrary host files, or consumer secrets.
12. Verify timeout/explicit cancellation removes or stops the workload and cleanup removes network/container.
13. Verify no orphan remains after normal/error test paths.

If a host cannot support required rootless resource enforcement, mark the backend/profile unsupported on that host rather than skipping the required release claim.

## Future stronger-backend tests

### gVisor/containerd

- OCI config compatibility;
- lease/cleanup semantic parity;
- runtime identity attestation (`runsc`/configured RuntimeClass);
- workload functional compatibility;
- network/filesystem/namespace/isolation evidence;
- startup/throughput overhead relative to Podman baseline;
- documented unsupported syscalls/application classes.

### Windows VM detonation

- VM snapshot identity;
- no host credentials/devices/shares;
- bounded network and time;
- process/registry/file/network telemetry;
- teardown verification.

## Property and fuzz testing

- arbitrary byte artifacts never panic and never execute;
- bounded identifier/command/image parsing;
- resource-policy comparisons;
- port-output parsing;
- JSON deserialization with unknown fields rejected;
- deterministic sandbox identity for same inputs and no unsafe name characters.

## Coverage

Repository policy requires exact 100% owned production statement/function/region/branch coverage where tooling exposes it. Coverage must include error paths, not only happy-path public APIs. No source-exclusion workaround is accepted merely to satisfy the number.

Branch coverage may require a pinned nightly Rust toolchain; the workflow must invoke the intended nightly explicitly and prove that exact toolchain produced the report.

## Security and supply-chain gates

- Clippy `-D warnings`;
- rustdoc `-D warnings`;
- SAST/security/dependency/advisory gates;
- lockfile freshness;
- pinned actions;
- SBOM/provenance/reproducibility;
- immutable application test image digest;
- exact-current-head checks only.

## Consumer contract tests

### Wardnet

- same evidence contract can be consumed without runtime deciding verdict;
- incomplete dynamic analysis cannot become an allow verdict by runtime default;
- runtime evidence provenance remains intact.

### contextual-orchestrator

Owner issue #991 requires consumer-side tests for:

- unauthorized application rejected before launch;
- only allowlisted immutable digest accepted;
- no prompt/body/credential passthrough;
- returned endpoint must be loopback-attested and lease unexpired;
- task success/cancel/timeout/error all terminate lease;
- cleanup failure is surfaced;
- no direct Podman/containerd invocation in Agent domain code.

## Release acceptance

A release candidate uses one exact protected source head for all applicable tests, security scans, coverage, rustdoc, package/build, real container E2E, SBOM/provenance, independent review, rollback/recovery and operational evidence. Evidence from an earlier head is not transferable.
