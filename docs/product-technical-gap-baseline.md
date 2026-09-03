# Product and Technical Gap Baseline

Last reviewed on 2026-09-03 KST from fresh GitHub state and the current candidate stack. This ledger distinguishes protected/default-branch truth, active-PR implementation, executable evidence, central control-plane dependencies, and immutable release authority. Volatile workflow/run identities are evidence snapshots, not durable architecture authority.

## Repository and governance truth

- Default/integration branch remains `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` at this review point.
- `main@6d27dba3d5b013b922e21757431330d048a91d70` remains the initialization-era line and is not the current release source.
- Organization ruleset `18156473` follows `~DEFAULT_BRANCH`; branch/review governance must be reread before every merge.
- GitHub Releases: **0**. Cargo version `0.1.0` is development metadata, not a shipped release.
- Queued, skipped, cancelled, absent, predecessor-head, static-only, local-only, or unassigned-runner results are non-passing evidence.

Current dependency order is:

```text
#1  runtime foundation                 @ d2893e39ef71d72d315f71f44ee04f498369b3e2
 -> #6  caller-scoped leases            @ eed831ded4da519d97df0d6e000682fe4b1c5d84
 -> #9  effective isolation             @ 8f13c1bab1582730715c5674e513e91148498ae5
 -> #10 release evidence                @ 91d7b20e1dd5053a95d1551ed9371d6e0b77aeb6
 -> #13 bounded command contract        @ 1667ba1169e5bd9ee17c87002e127200ab9e6baf
 -> #14 Podman command backend and CLI  @ current branch head
```

#10, #13, and #14 were restacked non-force after #9's effective-LSM repair and traceability update. The immediate #14 implementation parent before this baseline refresh is `e6ab46b8f3a17e0c673ab615ea458d18fe75f1dd`; the baseline commit itself advances the same leaf branch, so `git rev-parse HEAD` is the canonical exact leaf identity. No predecessor check or review evidence transfers across these movements. ADR-0006, ADR-0007, and ADR-0008 remain **Proposed** while their decisions are Draft/unmerged and lack protected-integration/current-head acceptance.

## Product responsibility and Context Map

Quarantine Sandbox Runtime is the canonical reusable isolation/evidence boundary. Core `sandbox_execution` owns backend-neutral isolation requirements and verified runtime state. Supporting `application_service` owns isolated service leases and bounded command execution. Supporting `artifact_analysis` owns artifact identity and analysis evidence contracts. Podman/gVisor/containerd/Kubernetes/VM mechanics stay behind infrastructure ports/ACLs.

Wardnet retains gateway/SOC policy, maliciousness verdicts, incidents, quarantine/block/review, notification, and retention. `contextual-orchestrator` retains model/Agent orchestration, caller authorization, application selection, secrets, tasks, and user-visible actions. Consumers must use released versioned contracts/ACLs; sibling source, mutable branch dependencies, backend SDK leakage, and cross-service application-table SQL are forbidden.

Target bounded contexts remain Workload Admission, Isolation Policy, Runtime Provisioning, Network/Egress, Artifact Analysis, Evidence/Provenance, Session Lifecycle, and Recovery. Further package extraction must follow aggregate/transaction ownership rather than technical-directory fan-out.

## Application-service isolation

| Capability | Current evidence | Maturity / next action |
| --- | --- | --- |
| Immutable workload identity | Digest-pinned lowercase SHA-256 OCI image references; no implicit pull. | Active-PR implementation; preserve through admission/release. |
| Rootless execution | Podman host state is inspected; non-rootless execution fails closed. | Active-PR implementation; exact-release-host proof required. |
| Filesystem/identity/namespace isolation | Read-only rootfs, bounded `noexec,nosuid,nodev` tmpfs, ignored image volumes, non-root UID/GID, isolated user/PID/IPC namespaces. | Active-PR implementation; maintain real-backend hostile regressions. |
| Capability/no-new-privileges | Container effective/bounding plus process effective/bounding/inheritable/permitted/ambient capability sets are required empty; no-new-privileges required. | Active-PR implementation; positive exact-head runtime proof required. |
| Seccomp / LSM | Host support alone is insufficient. Live process seccomp and AppArmor/SELinux security context are required. AppArmor inspect profile `containers-default` and process context `containers-default (enforce)` are treated as the same explicit enforcing profile; `(complain)`, `unconfined`, empty, malformed, contradictory, and mismatched profiles fail closed. | Causal source repair and paired positive/negative regressions are on #9; exact-head execution plus positive LSM runner proof remains required. |
| Network / publication | Service profile uses per-sandbox internal DNS-disabled network and loopback-only publication; command profile uses `--network none`. | Active-PR implementation; controlled egress requires a separate reviewed profile. |
| Resource bounds | CPU/RAM/PID/tmpfs/lifetime/readiness/shutdown limits are validated and inspected. | Active-PR implementation; revalidate on the supported cgroup/runtime profile. |
| Caller ownership / idempotency | #6 scopes lease state by caller owner + request identity, rejects changed immutable request/policy reuse, and bounds cleanup work. | Active-PR implementation; authenticated transport and durable replay remain missing. |
| Cleanup | Service and command paths attempt runtime-owned cleanup; post-create failures route through cleanup and cleanup failure dominates when resource removal cannot be proven. | Source/tests now contain fail-closed cleanup precedence and backend-loss regressions; exact-head CI remains required before GREEN. |
| Crash/restart recovery | No durable lease journal/orphan reconciliation. | Buyer gap: durable Session Lifecycle/Recovery contract plus crash/orphan E2E. |
| Stronger backends | No production gVisor/containerd/microVM adapter. | Add only behind the same verified isolation ACL after the P0 contract stabilizes. |

### Exact current runtime/security evidence

#9 exact head `8f13c1ba...` has CI run `33729962482`. Jobs `branch-coverage` `100567404376`, positive-LSM `100567404489`, `coverage` `100567404527`, negative-rootless-AppArmor `100567404629`, and `verify` `100567404669` are all queued before checkout with `steps=[]`; each exposes no assigned runner. Hosted jobs request `ubuntu-24.04`; the positive lane requests `[self-hosted, linux, cwl-hostile-workload, selinux]`. This is non-passing admission/capacity evidence.

The immediate #14 implementation parent `e6ab46b8...` has CI run `33730158137`. Positive-LSM `100568007580`, branch coverage `100568007892`, verify `100568007894`, coverage `100568007946`, and negative-rootless-AppArmor `100568007983` are likewise queued before checkout with no runner identity. `.github#712` owns the hosted/pre-checkout admission class; `.github#1590` owns the reviewed positive-LSM runner profile. Product source must not churn merely to retrigger either class.

## Effective LSM causal repair

A real contract mismatch was repaired on #9 rather than reported as a permanent blocker. Podman container inspection can expose `AppArmorProfile=containers-default` while the effective task security context includes the mode suffix `containers-default (enforce)`. The runtime now normalizes only an explicit enforcing suffix before comparing it to the inspect-reported profile. A paired negative regression proves `(complain)` is not confinement. SELinux continues to require exact non-empty, non-`unconfined` process-label equality.

Doctoring now ties this decision to the Linux LSM task-security-context interface and AppArmor's Enforce/Complain semantics. The source repair is not release proof: exact-head tests and a reviewed real positive-LSM backend still must execute.

## Bounded command execution

PR #13 proposes the provider-neutral run-to-completion contract: `CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, and `execute_command`. Nonzero workload exit status is structured workload evidence, not a sandbox malfunction. The contract reuses existing digest/image, isolation-policy, and resource-bound types rather than creating a second isolation authority.

PR #14 proposes the production rootless-Podman backend and synchronous `quarantine-sandbox-runtime run` CLI. It uses the P0 isolation policy, `--network none`, bounded Podman invocation/output, `podman wait` for authoritative completion, retained bounded logs, and fail-closed cleanup.

The earlier static-inspect fallback for fast-exiting commands remains rejected. `verify_command_isolation` requires live `podman top` process evidence for seccomp, LSM, and privilege-relevant capability sets; a process that exits before attestation currently fails closed. Future short-command support requires a reviewed start/hold/attest/release handshake or stronger backend primitive, not weaker static evidence.

The command path now checks log-command invocation/timeout/output/status errors and routes post-create failure through cleanup. Regressions cover cleanup failure after otherwise successful execution and backend loss during wait/log retrieval, with `CleanupFailed` taking precedence when removal cannot be proven. These are current source/test contracts, not remote GREEN until exact-head CI executes.

## Artifact analysis

| Capability | Current state | Next buyer/security slice |
| --- | --- | --- |
| Immutable static evidence foundation | SHA-256 artifact identity, bounded source context, deterministic classification, ordered analyzer port, attributable failures on #1. | Integrate foundation and bind immutable release identity. |
| Verdict boundary | Runtime emits risk/analysis evidence; consumer retains business/security disposition. | Preserve; never promote risk score/verdict into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF adapters | Missing. | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux dynamic detonation | Missing. | Reuse verified sandbox or a stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | Missing. | Separate Windows isolation pool preserving the common evidence contract. |
| Controlled network telemetry | Missing. | Add explicit sinkhole/approved-egress policy and bounded capture; no production credentials. |
| Durable chain of custody | Process-local only. | Add immutable object storage/signing/retention/replay and recovery ownership before GA. |

## Release delivery

PR #10 turns the zero-release state into an executable release candidate: protected-source/tag identity, locked Cargo packaging, complete owned coverage gates, SPDX SBOM, SHA-256 manifest, provenance, reproducibility, rollback expectations, and immutable GitHub Release assets. It remains Draft and is not release authority.

The release-attestation contract has already driven a causal candidate repair: before `gh release create`, the release job must verify provenance for the exact `.crate` against `ContextualWisdomLab/quarantine-sandbox-runtime` and independently verify the SPDX 3 predicate for those package bytes. That implementation has been non-force carried through the current stack. It still requires exact-head execution; creating or verifying attestations on a predecessor head is not transferable evidence.

No external registry publication is currently authorized. The first approved distribution path remains a GitHub Release only after one exact integrated protected source identity passes product/security/positive-LSM/coverage/package/SBOM/provenance/reproducibility/review/rollback gates together. No release exists today.

## Consumer compatibility and handoff

Production consumers must pin a future immutable released artifact/version plus checksum and verified provenance/SBOM. Open PR heads, branch names, semantic-version labels, unsigned tags, and transient Actions artifacts are not production dependencies.

| Consumer | Authority retained by consumer | Required handoff |
| --- | --- | --- |
| Wardnet | Gateway/SOC policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention. | Consume released artifact-analysis evidence contract only. |
| contextual-orchestrator | LLM/Agent orchestration, authorization, application selection, secrets, task/user actions. | Consume released application-service/command contract through an ACL; never direct Podman/containerd or sibling source. |
| `.github` central review | Review verdict and executed-PoC evidence policy. | After immutable command-runtime release, migrate sandboxed verification through the released CLI/API contract rather than copying runtime policy. |

## Context Graph and Enterprise Architecture read-only dependency state

`ContextualWisdomLab/context-graph-contracts` remains the contract-only Shared Kernel and still has no immutable release. Fresh open-PR inventory confirms root #4 remains Draft at `03caa05e432a46227e16ecddd61ed825d1a104dd`; release-source prerequisite #25 is Draft at `9283d7b8ed85b97b893eeacf339524177f3ffbfc`; DDD baseline #20 is `475ce14185db697940e8219c3cda7f24d66f3ed7`; Context Assertion/CloudEvent #21 is `b0c21f907a12b07a28cf38ed165ab6530855283e`. The intended release/projection rebuild remains dependency-first around `#19 -> #25 -> #20 -> #21`. No open CGC head is production interoperability authority.

`ContextualWisdomLab/enterprise-architecture-core` also has no immutable release. Fresh open-PR inventory confirms Draft #40 remains the quarantine/Context Fabric consumer projection owner at `3734f0c8d2533015c2b2cbfe22d16f1a507881cb`; its recorded producer snapshot is now stale because the quarantine stack advanced. #40 must be updated only by the dedicated Context Fabric writer after released CGC authority exists. Allowed projection remains runtime/backend identity, technology/provider/version, lifecycle/risk context, ownership, remediation/transformation, and attestation provenance. Malware verdict/artifact risk score, foreign DB access, and source copying remain prohibited.

The dedicated Context Fabric writer owns CGC/EA source and PR-state repair. This writer supplies exact read-only producer evidence/acceptance criteria only.

## Highest-leverage buyer-visible gaps

1. Restore exact-head hosted runner admission through `.github#712`; execute the current tests instead of manufacturing source churn.
2. Provide disposable positive-LSM runtime capacity through `.github#1590` and bind effective LSM/seccomp/capability/resource/cleanup evidence to one exact candidate.
3. Drain `#1 -> #6 -> #9 -> #10 -> #13 -> #14` by exact RED→causal GREEN→normal merge→non-force descendant restack, preserving unique deltas and reacquiring evidence after every movement.
4. Complete the first immutable release from one protected integrated source identity, then bump Wardnet/contextual-orchestrator/central-review consumers to that released contract.
5. Add durable authenticated admission/session/recovery/idempotency plus orphan reconciliation for remote/multi-process operation.
6. Add real artifact analyzers and dynamic detonation profiles with immutable chain-of-custody evidence rather than expanding consumer verdict authority into this runtime.

No item above is complete from documentation, queued workflows, local-only results, or open-PR artifacts alone.