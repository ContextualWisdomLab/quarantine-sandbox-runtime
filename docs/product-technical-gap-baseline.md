# Product and Technical Gap Baseline

Last reviewed on 2026-09-05 against live GitHub state. Protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` remains shipped authority. The active Draft dependency chain is `#1@c6422fa0a2ce51521fb7e82e77b5023bfdd7dfef -> #6@6c8aac828fe1d0cddb4d5ea7890783a4118c6328 -> #9@67a04dc47b8c1bbe61af6a89bef09c551393bd84 -> #10@013812b903579551b960314306ffd4815728725c -> #13@459ef5d2acd679a8f6c328199d8958f6dd02585d -> #14 source/restack@8f08fa854b2892b456394f45dad59e1eb08b20d3`. The commit containing this ledger update is documentation-only. Queued, skipped, cancelled, stale, predecessor-head, or pre-checkout results are non-passing evidence.

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
| Immutable workload identity | Registry/storage image names are digest-pinned; host-path/alternate-store `containers/image` transports are rejected. | Implemented on #1 | Keep image admission/import separate from launch and prohibit implicit pulls. |
| Rootless backend | Podman host state is inspected and non-rootless execution fails closed. | Implemented | Re-run on every release head. |
| Filesystem isolation | Read-only rootfs, bounded `noexec,nosuid,nodev` tmpfs, image volumes ignored, no arbitrary host mounts. | Implemented | Add only typed reviewed read-only inputs when a buyer flow requires them. |
| Effective privilege/seccomp/LSM proof | The active stack verifies live process seccomp, effective/bounding/inheritable/permitted/ambient capability sets, non-root identity, namespaces and enforcing AppArmor/SELinux evidence instead of trusting launch argv. | Implemented candidate; exact-head release proof pending | Preserve fail-closed behavior and obtain the dedicated positive-LSM evidence before release. |
| Resource enforcement proof | CPU/RAM/PID inspect values exist, but #19 retains the P0 RED for applied `/tmp` restrictions, timeout and live enforcement proof. | RED-only descendant | Execute the RED for its intended reason before the smallest causal production repair. |
| Effective network binding | #23 retains the P0 RED requiring the running service container to be attached only to the exact runtime-owned internal network, not merely proving that a safe network object exists. | RED-only descendant | Execute the RED, then verify authoritative effective attachment and exact cleanup ownership. |
| Caller ownership/idempotency | #6 scopes service lease ownership/idempotency by command context, rejects changed request/policy reuse, prevents wrong-owner cleanup and bounds/fairly retries expiry cleanup. | Implemented candidate | Bind owner to authenticated transport and add durable replay/admission/recovery. |
| Runtime resource identity | #21 retains a P0 RED requiring independent invocations with the same consumer request/start second to own distinct container/network/label identities and exact cleanup targets. | RED-only descendant | Execute the collision RED before adding a collision-resistant runtime-generated invocation identity. |
| Crash/restart orphan reclamation | No durable lease journal or orphan reconciliation. | Missing | Add Session Lifecycle/Recovery contracts and crash/orphan E2E after the isolation foundation integrates. |
| Stronger backends | No production gVisor/containerd/VM/Kubernetes adapter. | Missing | Add only behind the same verified-isolation ACL after P0 and first release stabilize. |

## Shared bounded-command infrastructure repair

The dependency stack previously inherited a production `Command::spawn` retry on `io::ErrorKind::WouldBlock` from #6. The only executed root failure surfaced `BackendInvocationFailed { operation: "rootless_probe" }`; `BoundedCommandRunner` discarded the underlying OS error, so that evidence did not establish `WouldBlock` and there was no focused RED defining retry count, elapsed budget, or error precedence.

`#6@6c8aac828fe1d0cddb4d5ea7890783a4118c6328` therefore removes only that unproven retry while preserving the fixture-isolation delta. #9, #10, #13 and #14 adopted the repaired parent through non-force two-parent commits and explicitly replaced their historical `bounded_command` blob with the canonical foundation blob. This is a repair of an unsupported workaround, not evidence that transient spawn exhaustion cannot occur. If a future exact failure preserves a retryable errno, add a focused foundation RED first and define a bounded retry contract there.

## Bounded command execution

PR #13 owns the provider-neutral command request/result/backend contract and keeps nonzero workload exit status distinct from sandbox/runtime failure. PR #14 adds the production `RootlessPodmanAdapter::run_command_at` implementation plus the `quarantine-sandbox-runtime run` CLI.

The command backend creates a fresh digest-pinned, read-only-rootfs, non-root, bounded sandbox with `--network none`, dropped capabilities, no-new-privileges and isolated namespaces. It starts detached, then requires live `podman top` evidence for effective seccomp, LSM and all privilege-relevant capability sets before accepting the workload. If a short-lived command exits before live evidence can be sampled, execution fails closed; static `container inspect` configuration is not promoted to observed runtime evidence.

PR #14 retains invocation-unique one-shot `qsr-cmd-*` identities, Podman-4.9-compatible create argv without `--no-hostname`, bounded per-stream output, effective `NetworkMode == none` inspection, and bounded exact-revision source staging into a runtime-owned read-only `noexec,nosuid,nodev` tree. Historical command identity / Podman-4.9 RED head `9dc2e4e41aadbae7762d45c54cb007ff7515a399` did not execute before production candidate `1f83c4c50bb1e5ded250c155cb5c1ef383e8b762`; that sequence remains evidence debt rather than a completed RED->GREEN claim.

ADR-0007 and ADR-0008 remain Proposed until protected integration with current evidence.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Immutable SHA-256 identity, bounded source context, deterministic format classification, ordered analyzer port and attributable failures. | Implemented on #1 | Integrate and retain exact release evidence. |
| Claude plugin package profile | #18 defines a strict contract-first RED for immutable repository/tree identity, bounded probe set/resources and path/control-text validation. | RED-only descendant | Execute the exact RED before adding the smallest profile-specific production contract. |
| Verdict boundary | Evidence/disposition remain risk/analysis evidence and require a consumer verdict. | Implemented | Never promote analyzer output into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF | No production adapters. | Missing | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux/Windows detonation | No production detonation pool. | Missing | Reuse the verified boundary or stronger gVisor/microVM/Windows isolation while preserving evidence contracts. |
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
| Cargo source package / SPDX SBOM / SHA-256 manifest / provenance | workflow candidates only |
| Byte reproducibility | candidate workflow rebuilds locked package and compares bytes before publication |
| Positive hostile-runtime proof | requires reviewed SELinux-capable rootless runtime infrastructure; generic hosted negative evidence is insufficient |

PR #10 also retains the release-authority RED: the repository default branch is `develop` while historical release preflight logic hard-coded unprotected `main`. The RED must execute before production workflow logic is changed to bind a tag to the live protected default-branch authority.

## Consumer compatibility and handoff

Strict consumers validate exact schema versions and pin a released package plus SHA-256 and verified provenance/SBOM evidence. PR heads, branch refs, semantic-version labels and transient Actions artifacts are not production identities.

| Consumer | Authority retained by consumer | Integration state |
| --- | --- | --- |
| Wardnet | SOC/gateway policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention | Must consume released artifact-analysis evidence through its owner ACL. |
| contextual-orchestrator | model/Agent orchestration, authorization, application selection, secrets, task/user actions | Owner path remains downstream of an immutable runtime release; direct Podman/containerd calls and sibling source are forbidden. |

## Verification and governance state

- #1 exact `c6422fa0a2ce51521fb7e82e77b5023bfdd7dfef` remains Draft/mergeable. CI `33880554497`, Security Scan `33880554610`, SAST `33880554601`, and CodeQL PR `33880555199` are materialized but still queued at this review; no result transfers from predecessor heads.
- #6 exact `6c8aac828fe1d0cddb4d5ea7890783a4118c6328` preserves lease ownership/fixture isolation and removes the unproven shared spawn retry. Its fresh Actions runs had not materialized at the first post-push read.
- #9 exact `67a04dc47b8c1bbe61af6a89bef09c551393bd84`, #10 exact `013812b903579551b960314306ffd4815728725c`, #13 exact `459ef5d2acd679a8f6c328199d8958f6dd02585d`, and #14 source/restack exact `8f08fa854b2892b456394f45dad59e1eb08b20d3` are non-force descendants of the repaired parent path and no longer contain the unsupported retry delta.
- #18, #19, #21 and #23 remain Draft RED lanes and cannot merge ahead of their prerequisite protected ancestry. Their current REDs must execute for the intended reason before production GREEN.
- `.github#712` owns generic hosted-runner admission. `.github#1590` separately owns disposable SELinux-capable positive effective-LSM capacity. These are distinct evidence paths.
- Organization ruleset `18156473` remains the protected-branch authority; bypass capability is not merge evidence.
- No immutable release exists.

## Next bounded slices

1. Obtain fresh exact-head execution evidence dependency-root first; do not churn clean source solely to retrigger queued jobs.
2. If the root `BackendInvocationFailed { operation: "rootless_probe" }` recurs, preserve the actual OS error before considering any retry policy; do not restore the removed generic `WouldBlock` workaround without a focused RED.
3. Execute the #19 resource, #21 runtime-identity, #23 effective-network-binding and #18 artifact-profile REDs on correctly reconciled ancestry before production repair.
4. Drain exact-head product/security/coverage/review gates dependency-root first; use normal merge followed by non-force descendant adoption.
5. Obtain positive effective LSM/seccomp/capability/resource/cleanup evidence on the reviewed security runner profile without weakening P0 for generic hosted capacity.
6. Publish `0.1.0` only from one exact integrated protected head with dated CHANGELOG, immutable release assets, SBOM/provenance/reproducibility and rollback evidence.
7. Update Wardnet/contextual-orchestrator owner paths to pin released artifact SHA-256/provenance and exact schema versions.
8. Add durable Workload Admission, Session Lifecycle and Recovery contracts with crash/replay/resource-reservation E2E, then stronger isolation backends and artifact detonation profiles.
