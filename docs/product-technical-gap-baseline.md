# Product and Technical Gap Baseline

Last reviewed on 2026-09-04 against the live dependency stack. Protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` remains shipped authority. The active Draft stack is now `#1@3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3 → #6@89103472cea8f27661614e4f4740e68d2f4a153b → #9@64b1ba4f202843288e9a9c4b104e0f93aad76f43 → #10@30a2c2080bc1d2e8d6d049b70477721dccb4d8dc → #13@3a26a5c27e81fc315c98a88005b8154d2ca95b7f → #14 source/restack@e38537507773a005a74f693fc150004263cf9b49`. The ledger commit containing this file changes documentation/restack metadata only. Queued, skipped, cancelled, stale, predecessor-head, or pre-checkout results are non-passing evidence.

## Product responsibility and Context Map

Quarantine Sandbox Runtime is the reusable security boundary for two consumer profiles and one bounded execution surface:

- `application_service`: hostile/untrusted applications behind an isolated loopback service lease;
- `artifact_analysis`: immutable artifact identity plus static/dynamic risk evidence and provenance;
- bounded command execution inside the existing `application_service` Supporting context for CI/security consumers that need one isolated command result rather than a network service.

Core `sandbox_execution` owns backend-neutral isolation policy, resource bounds, runtime identity and verified effective isolation state. `application_service` owns service leases, caller-scoped ownership/idempotency and bounded command contracts. `artifact_analysis` owns analysis evidence, never consumer verdict authority. Podman/gVisor/containerd/Kubernetes/VM implementations remain infrastructure adapters behind ports/ACLs.

Wardnet retains gateway/SOC policy, maliciousness verdict, incident, quarantine/block/review, notification and retention. `contextual-orchestrator` retains LLM/Agent orchestration, caller authorization, application selection, secrets and user-visible actions. Consumer repositories use immutable released contracts only; sibling source vendoring, mutable-branch production dependency and cross-service application-table SQL are forbidden.

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable workload identity | Registry/storage image names must be pinned by lower-case SHA-256. Explicit `containers/image` transports such as `dir:`, `docker-archive:`, `oci-archive:`, `docker-daemon:` and `containers-storage:` are rejected even with a digest suffix. | Implemented on #1 and inherited | Keep image admission/import separate from launch; no host-path/alternate-store transports or implicit pulls. |
| Rootless backend | Podman host state is inspected and non-rootless execution fails closed. Backend version is evidence, not isolation proof. | Implemented | Re-run on every release head. |
| Filesystem isolation | Read-only rootfs, bounded noexec/nosuid/nodev tmpfs, image volumes ignored, no arbitrary host mounts. | Implemented | Add only typed reviewed read-only inputs when a buyer flow requires them. |
| Effective privilege/seccomp/LSM proof | #9 verifies live process seccomp, effective/bounding/inheritable/permitted/ambient capability sets, non-root identity, namespaces, resource limits and an enforcing AppArmor/SELinux identity rather than trusting launch argv. | Implemented downstream on #9 | The parent #1 contract cannot merge while it still creates an all-positive lease attestation without equivalent effective proof. |
| Parent attestation truthfulness | #1 exact `3fa5c549…` adds RED `tests/podman_effective_attestation_red.rs`: a fake rootless Podman can accept the configured launch plan and reach readiness while supplying no effective isolation evidence, and the runtime must fail closed instead of returning an all-positive lease. Production code was deliberately not changed before this RED executes. | RED queued; non-passing | Execute the exact RED, prove the old behavior fails, then fold the smallest effective-attestation repair into the canonical merge path without duplicating #9 logic. |
| Network isolation | Service profile uses per-sandbox internal DNS-disabled network plus loopback-only publication; command profile uses `--network none`. | Implemented on active stack | Controlled egress must be a separate versioned profile. |
| Credential isolation | P0 has no consumer/provider credentials, arbitrary environment, runtime sockets, host devices or ambient proxy inheritance. | Implemented | Future secrets require an explicit task-scoped broker and authorization contract. |
| Caller ownership/idempotency | #6 scopes service lease ownership/idempotency by command context, rejects changed request/policy reuse, prevents wrong-owner cleanup and bounds/fairly retries expiry cleanup. | Implemented on active PR | Bind owner to authenticated transport and add durable replay/admission/recovery. |
| Crash/restart orphan reclamation | No durable lease journal or orphan reconciliation. | Missing | Add `session_lifecycle`/`recovery` contracts and crash/orphan E2E after the isolation foundation is integrated. |
| Stronger backends | No production gVisor/containerd/VM/Kubernetes adapter. | Missing | Add only behind the same verified-isolation ACL after P0 and first release stabilize. |

## Bounded command execution

PR #13 owns the provider-neutral command request/result/backend contract and keeps nonzero workload exit status distinct from sandbox/runtime failure. PR #14 adds the production `RootlessPodmanAdapter::run_command_at` implementation plus the `quarantine-sandbox-runtime run` CLI.

The command backend creates a fresh digest-pinned, read-only-rootfs, non-root, bounded sandbox with `--network none`, dropped capabilities, no-new-privileges and isolated namespaces. It starts detached, then requires live `podman top` evidence for effective seccomp, LSM and all privilege-relevant capability sets before accepting the workload. If a short-lived command exits before live evidence can be sampled, execution fails closed; static `container inspect` configuration is not promoted to observed runtime evidence. Future support for extremely short-lived commands requires a reviewed start/hold/attest/release handshake or a stronger backend primitive.

PR #14 now gives one-shot resources invocation-unique identities while preserving the consumer correlation id. Its PR-source slice requires an exact revision plus expected tree digest, copies only bounded regular files into runtime-owned staging, rejects links/special files, strips executable bits, and mounts only the verified tree read-only with `noexec,nosuid,nodev`. The command result carries the exact-revision/tree receipt only after container inspection confirms those mount controls. Checkout authorization and exact-revision materialization remain consumer responsibilities; QSR owns sanitization, isolation, execution, and evidence.

The #14 restack deliberately keeps the current full ADR-0007 from #13 rather than inheriting an older child simplification that deleted causal context and alternatives. ADR-0008 remains the child decision for the Podman command backend/CLI. The old #14 CI `paths-ignore` delta is also not carried forward: exact-head product evidence must not disappear on documentation-only head movement merely to reduce queue pressure; central queue/admission is owned by `.github#712/#1796`.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Immutable SHA-256 identity, bounded source context, deterministic format classification, ordered analyzer port and attributable failures. | Implemented on #1 | Integrate and retain exact release evidence. |
| Verdict boundary | Evidence/disposition remain risk/analysis evidence and require a consumer verdict. | Implemented | Never promote analyzer output into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF | No production adapters. | Missing | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux dynamic detonation | No artifact detonation vertical yet. | Missing | Reuse the verified sandbox boundary or stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | No production pool. | Missing | Separate Windows isolation boundary while preserving common evidence contracts. |
| Controlled network telemetry | No sinkhole/approved-egress analysis profile. | Missing | Add explicit policy, bounded capture and no production credentials. |
| Durable evidence/chain of custody | Evidence is process-local. | Missing | Add immutable object storage, signing, retention/replay and recovery ownership before GA. |

## Release delivery

The absence of an immutable release remains an implementation gap. PR #10 contains the first fail-closed release-delivery contract and remains downstream of the product/security stack.

| Item | Current state |
| --- | --- |
| Package version | `0.1.0` development metadata; not a released identity |
| GitHub Releases | none |
| Stable release source | no integrated protected product release head yet |
| Application-service lease contract | `1.2.0` candidate |
| Artifact-analysis request/evidence | `1.0.0` candidate |
| Command request/result | `1.0.0` candidate |
| Release evidence contract | `1.0.0` candidate on #10 |
| Cargo source package | workflow candidate only |
| SPDX SBOM | workflow candidate only |
| SHA-256 manifest | workflow candidate only |
| Provenance/SBOM attestation | workflow candidate only |
| Byte reproducibility | candidate workflow rebuilds locked package and compares bytes before publication |
| Positive hostile-runtime proof | requires reviewed positive LSM-capable infrastructure; generic hosted negative evidence is insufficient |

The release path must reject a tag/version that disagrees with protected source, or any candidate lacking exact-head product/security/coverage/review/effective-isolation/positive-LSM/package/SBOM/provenance/reproducibility/rollback evidence. GitHub Release assets are the first configured immutable distribution channel; no crates.io authority is assumed.

## Consumer compatibility and handoff

`docs/contracts/consumer-contract.md` is the pre-release compatibility baseline. The #14 restack keeps the parent's registry-only image restrictions and effective-isolation semantics while documenting the production command backend truthfully: live process evidence is mandatory and short-lived workloads fail closed when that proof disappears.

Strict consumers validate exact schema versions and pin a released package plus SHA-256 and verified provenance/SBOM evidence. PR heads, branch refs, semantic-version labels and transient Actions artifacts are not production identities.

| Consumer | Authority retained by consumer | Integration state |
| --- | --- | --- |
| Wardnet | SOC/gateway policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention | Must consume released artifact-analysis evidence through its owner ACL. |
| contextual-orchestrator | model/Agent orchestration, authorization, application selection, secrets, task/user actions | Owner path #991 remains downstream of an immutable runtime release; direct Podman/containerd calls and sibling source are forbidden. |

## Context Graph and Enterprise Architecture read-only integration

`ContextualWisdomLab/context-graph-contracts` is the contract-only Shared Kernel for canonical authority/object references, truth status/origin, bitemporal semantics, provenance, Context Assertion, CloudEvents and conformance/admission. `ContextualWisdomLab/enterprise-architecture-core` owns authoritative EA decisions. This writer does not mutate either repository while the Context Fabric writer is active and does not consume their mutable PR heads as production dependencies.

After compatible immutable releases exist, runtime/backend identity, technology/provider/version, lifecycle, architecture-risk context, ownership, remediation/transformation and attestation provenance may flow through released contracts. Malware verdicts and artifact risk scores remain risk evidence and are not EA authoritative facts.

## Verification and governance state

- #1 exact `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` is Draft. CI `33800321670`, Security Scan `33800321674`, SAST `33800321679`, Scorecard `33800321680` and OSV `33800322168` are materialized but queued; the new effective-attestation RED has not executed.
- #6 non-force restack head `89103472cea8f27661614e4f4740e68d2f4a153b` inherits the moved parent while preserving caller ownership/idempotency/cleanup-fairness delta.
- #9 non-force restack head `64b1ba4f202843288e9a9c4b104e0f93aad76f43` preserves strict effective-isolation/LSM logic on the current #6 parent.
- #10 non-force restack head `30a2c2080bc1d2e8d6d049b70477721dccb4d8dc` preserves release evidence on the current isolation head.
- #13 non-force restack head `3a26a5c27e81fc315c98a88005b8154d2ca95b7f` preserves the bounded command contract on the current release head.
- #14 source/restack merge `e38537507773a005a74f693fc150004263cf9b49` preserves its command backend/CLI/test delta on current #13 while keeping parent security fixes. A following ledger/restack commit carries the corrected ADR/consumer/baseline resolution; predecessor review/check evidence does not transfer.
- Issue #16's command sandbox-identity defect is repaired on PR #14: repeated consumer correlation within the same start second receives distinct runtime resource identities, with cleanup ownership covered by the executable regression test. Protected exact-head integration remains pending.
- `.github#712/#1796` own organization queue/admission and duplicate queue-hygiene repair. Leaf source is not churned solely to manufacture runner assignment. Positive LSM capacity remains a separate reviewed infrastructure capability.
- Organization ruleset `18156473` is active on the default branch and requires one approving review, thread resolution and central required workflows. Legacy branch-protection output alone is not the authority; bypass capability is not merge evidence.
- No immutable release exists.

## Next bounded slices

1. Execute #1 effective-attestation RED and repair the canonical parent/successor boundary only after the old behavior demonstrably fails.
2. Preserve issue #16's collision-resistant runtime execution-instance identity and cleanup-ownership regression through protected integration.
3. Drain exact-head product/security/coverage/review gates dependency-root first; normal merge only, followed by immediate non-force descendant restack.
4. Obtain positive effective LSM/seccomp/capability/resource/cleanup evidence on the reviewed security runner profile without weakening P0 for generic hosted capacity.
5. Publish `0.1.0` only from one exact integrated protected head with dated CHANGELOG, immutable release assets, SBOM/provenance/reproducibility and rollback evidence.
6. Update Wardnet/contextual-orchestrator owner paths to pin released artifact SHA-256/provenance and exact schema versions.
7. Add durable Workload Admission, Session Lifecycle and Recovery contracts with crash/replay/resource-reservation E2E.
8. Add artifact-analysis adapters and detonation only on the released isolation boundary.
