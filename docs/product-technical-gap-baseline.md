# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 against the live dependency stack. Protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` remains shipped authority. The current active stack is `#1@06b39670ba5a434e8e34aac6f5b7fa2b6b75fe87 → #6@1cdfd29a6492405007167a17f5c7feefdd1eaa98 → #9@35549820adb40c2dda55bee4c3ea018ba1c664e8 → #10@24da0dd4006c753a235a70f2e5596a274e3f9196`. This ledger commit changes documentation only; any later production/source movement invalidates exact-head observations until refreshed. Queued, skipped, cancelled, stale, predecessor-head, or unassigned-runner results are non-passing evidence.

## Product responsibility and Context Map

Quarantine Sandbox Runtime is the reusable security boundary for two consumer profiles:

- `application_service`: short-lived hostile or untrusted application execution behind an isolated service lease;
- `artifact_analysis`: immutable artifact identity plus static/dynamic analysis evidence and provenance.

The bounded-context target remains:

```text
Workload Admission
Isolation Policy
Runtime Provisioning
Network / Egress
Artifact Analysis
Evidence / Provenance
Session Lifecycle
Recovery
```

Core `sandbox_execution` owns backend-neutral isolation requirements and verified runtime state. `application_service` owns caller-scoped lease/idempotency semantics. Infrastructure adapters own Podman/gVisor/containerd implementation details. Wardnet retains gateway/SOC policy, maliciousness verdict, incident, quarantine/block/review, notification, and retention authority. `contextual-orchestrator` retains LLM/Agent orchestration, caller authorization, application selection, secrets, and user-visible actions.

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable workload identity | PR #1 now rejects explicit container-image transports such as `dir:`, `docker-archive:`, `oci-archive:`, `docker-daemon:`, and `containers-storage:` even when a digest suffix is present. Only normal registry/storage names with a lower-case SHA-256 digest and optional numeric registry port are accepted. The request schema, consumer contract, hostile regression, and transport-reference traceability are aligned and inherited through #6/#9/#10. | Implemented on active stack; exact-head proof required | Keep image import/admission separate from launch. Never reintroduce host-path/alternate-store transports or implicit pulls. |
| Rootless backend | Podman host state is inspected and non-rootless execution fails closed. The current GitHub-hosted negative-LSM lane explicitly expects Ubuntu 24.04 distribution Podman 4.9.3; backend version is evidence, not proof of isolation. | Implemented | Require the same effective proof on every release head. |
| Filesystem isolation | Read-only rootfs, bounded noexec/nosuid/nodev tmpfs, image volumes ignored, no arbitrary host mounts, and no host-backed image transports. | Implemented | Add only typed reviewed input mounts if a buyer flow requires them. |
| Privilege isolation | Effective/bounding capability sets, no-new-privileges, non-privileged mode, numeric non-root identity, and namespace state are inspected after start. | Implemented on #9 | Preserve real hostile process-boundary regressions. |
| Seccomp | Host/runtime seccomp must be positively verified; unconfined/unknown state fails closed. | Implemented on #9 | Prove on the exact release runtime. |
| LSM | AppArmor/SELinux availability and effective per-sandbox confinement are separate facts. Bare/empty/complain/unconfined/mismatched evidence is not positive proof. | Product logic implemented; positive release proof requires dedicated capability | `.github#1590` or a reviewed stronger backend must provide disposable LSM-capable security infrastructure. Do not downgrade the profile for generic hosted runners. |
| Network isolation | Per-sandbox internal DNS-disabled network, denied external egress, and loopback-only service publication. | Implemented | Controlled egress must be a separate versioned profile. |
| Credential isolation | P0 has no consumer/provider credentials, arbitrary environment, runtime sockets, host devices, or ambient proxy inheritance. | Implemented | Future secrets require a task-scoped broker and explicit authorization contract. |
| Resource limits | Memory, CPU, PID, lease duration, tmpfs, readiness and shutdown bounds are validated and inspected. | Implemented on #9 | Re-run on every supported release host/cgroup profile. |
| Runtime identity | Lease schema `1.2.0` carries backend id/version, policy SHA-256, endpoint/timestamps, and effective control evidence. | Implemented on #9 | Bind released source/package identity and durable signed evidence. |
| Caller ownership/idempotency | #6 scopes lease ownership/idempotency by authenticated command context, rejects changed request/policy reuse, prevents wrong-owner cleanup, and bounds/fairly retries expired cleanup. | Implemented on #6 | Bind owner to an authenticated transport; add durable replay/admission and recovery. |
| Crash/restart orphan reclamation | No durable lease journal or orphan reconciliation. | Missing | Extract durable `session_lifecycle`/`recovery` contracts and add crash/orphan E2E. |
| gVisor/containerd/VM/Kubernetes | No production adapter. | Missing | Add only behind the same verified isolation ACL after P0/release stabilizes. |

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Immutable SHA-256 identity, bounded source context, deterministic format classification, ordered analyzer port and attributable failures. | Implemented on #1 | Integrate and retain exact release evidence. |
| Verdict boundary | Evidence/disposition remain risk/analysis evidence; consumer verdict is required. | Implemented | Never promote evidence score/verdict into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF | No production adapters. | Missing | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux dynamic detonation | No artifact detonation vertical yet. | Missing | Reuse the verified sandbox boundary or stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | No production pool. | Missing | Separate Windows isolation boundary preserving common evidence contracts. |
| Controlled network telemetry | No sinkhole/approved-egress analysis profile. | Missing | Add explicit policy, bounded capture and no production credentials. |
| Durable evidence/chain of custody | Evidence is process-local. | Missing | Add immutable object storage, retention/signing/replay and recovery ownership before GA. |

## Release delivery

The absence of a release remains an implementation gap. PR #10 provides the first fail-closed release-delivery contract while remaining downstream of the product/security stack.

| Item | Current state |
| --- | --- |
| Package version | `0.1.0` development metadata; not a released identity |
| GitHub Releases | none |
| Stable release source | no integrated protected product release head yet |
| Application-service lease contract | `1.2.0` |
| Artifact-analysis request/evidence contract | `1.0.0` |
| Release evidence contract | `1.0.0` candidate on #10 |
| Cargo source package | candidate workflow only; no released bytes |
| SPDX SBOM | candidate workflow only |
| SHA-256 manifest | candidate workflow only |
| GitHub provenance/SBOM attestation | candidate workflow only |
| Byte reproducibility | candidate workflow builds the locked package twice and compares bytes before publication |
| Positive hostile-runtime release proof | requires the reviewed LSM-capable lane; generic hosted negative evidence is insufficient |

The release workflow must reject any tag whose version disagrees with `Cargo.toml`, whose source is not the protected release head, or whose current product/security/coverage/positive-LSM/package/SBOM/provenance/reproducibility/review/rollback evidence is incomplete. GitHub Release assets remain the first configured immutable distribution channel; no crates.io publication authority is assumed.

## Consumer compatibility and handoff

`docs/contracts/consumer-contract.md` is the pre-release compatibility baseline. Strict consumers validate exact supported schema versions and pin a released package plus SHA-256 and verified provenance/SBOM evidence. A PR head, branch, tag alone, semantic-version label, or transient Actions artifact is not production identity.

| Consumer | Authority retained by consumer | Integration state |
| --- | --- | --- |
| Wardnet | SOC/gateway policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention | Must consume the released artifact-analysis evidence contract through its owner ACL. |
| contextual-orchestrator | model/Agent orchestration, authorization, application selection, secrets, task/user actions | Issue #991 remains the owner path; direct Podman/containerd calls and sibling source are forbidden. |

## Context Graph and Enterprise Architecture read-only integration

`ContextualWisdomLab/context-graph-contracts` is the contract-only Shared Kernel for canonical authority/object references, truth status/origin, bitemporal semantics, provenance, Context Assertion, CloudEvents and conformance/admission. `ContextualWisdomLab/enterprise-architecture-core` owns authoritative EA decisions. This quarantine writer does not mutate either repository while the Context Fabric writer is active and never consumes their mutable PR heads as production dependencies.

After compatible immutable releases exist, quarantine runtime/backend identity, technology/provider/version, lifecycle, architecture-risk context, ownership, remediation/transformation and attestation provenance may flow through released contracts. Malware verdicts and artifact risk scores remain risk evidence and are never copied into EA authoritative facts. Cross-service application-table SQL remains forbidden.

## Verification and governance state

- #1 current exact head `06b39670ba5a434e8e34aac6f5b7fa2b6b75fe87` is ahead of protected `develop` with `behind_by=0`, remains Draft, and has queued CI/security/SAST/Scorecard/OSV evidence. CI jobs are pre-checkout with no assigned runner identity at the latest read.
- #6 current exact head `1cdfd29a6492405007167a17f5c7feefdd1eaa98` was non-force restacked on #1; compare proves the current parent is its exact merge base and preserves only caller-ownership/idempotency/cleanup-fairness delta.
- #9 current exact head `35549820adb40c2dda55bee4c3ea018ba1c664e8` was non-force restacked on #6 and inherits the registry-only image contract, concrete bounded-process tests, hosted Podman 4.9.3 negative-LSM contract, and transport traceability while preserving strict effective-isolation/LSM logic.
- #10 source/restack head `24da0dd4006c753a235a70f2e5596a274e3f9196` is based on current #9 and remains Draft. Exact-head CI `33798868479` is queued and therefore non-passing.
- Central `.github#712` remains the runner-admission/queue-health owner; `.github#1590` remains the positive hostile-workload LSM-capability owner. Leaf source is not churned merely to manufacture assignment.
- Current rules/review requirements and unresolved-thread state remain merge authority; bypass capability is not routine evidence.
- No immutable release exists. Release/version publication remains blocked until one integrated protected head satisfies product/security/coverage/effective runtime/positive LSM/review/package/SBOM/provenance/reproducibility/rollback evidence together.

## Next bounded slices

1. Finish non-force descendant reconciliation from #10 through #13/#14 and regenerate each exact-head evidence without transferring predecessor results.
2. Obtain terminal exact-head product/security/coverage results for #1 → #6 → #9 → #10; fix the first actual causal failure when jobs execute.
3. Provision/validate the positive LSM-capable security lane through its canonical owner path, without weakening the product profile.
4. Integrate dependency order only through then-live protection, then promote `0.1.0`, dated CHANGELOG, immutable GitHub Release, SBOM/provenance and rollback evidence from one exact protected release head.
5. Update Wardnet/contextual-orchestrator owners to pin the released artifact SHA-256/provenance and exact schema versions.
6. Add durable Workload Admission, Session Lifecycle and Recovery contracts with crash/replay/resource-reservation E2E.
7. Add artifact-analysis adapters and dynamic detonation only on top of the released isolation boundary.
