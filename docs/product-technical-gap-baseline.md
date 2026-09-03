# Product and Technical Gap Baseline

Last reviewed on 2026-09-03 KST from fresh GitHub state and the current candidate stack. This ledger separates protected/default-branch truth, active-PR implementation, executable evidence, central control-plane dependencies, and immutable release authority. Workflow/run identifiers are evidence snapshots, not durable architecture authority.

## Repository and governance truth

- Default/integration branch remains `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` at this review point.
- GitHub Releases: **0**. Cargo version `0.1.0` is development metadata, not a shipped release.
- Organization ruleset `18156473` follows `~DEFAULT_BRANCH`; branch/review governance must be reread before every merge.
- Queued, skipped, cancelled, absent, predecessor-head, static-only, local-only, or unassigned-runner results are non-passing evidence.

Current dependency order is:

```text
#1  runtime foundation                 @ d2893e39ef71d72d315f71f44ee04f498369b3e2
 -> #6  caller-scoped leases            @ eed831ded4da519d97df0d6e000682fe4b1c5d84
 -> #9  effective isolation             @ 693ee6c096b53f562fb106a3bcb9fb9efa092c8f
 -> #10 release evidence                @ fce2413c9d32bf0363bb38ff7669d6f4ee312738
 -> #13 bounded command contract        @ db5433781be679e7a4239f88b9c435ad7bd7f64c
 -> #14 Podman command backend and CLI  @ current branch head
```

The stack was converged non-force after the effective-LSM repair. No predecessor check/review evidence transfers across head/base movement. ADR-0006, ADR-0007, and ADR-0008 remain **Proposed** while unmerged and without protected-integration/current-head acceptance.

## Product responsibility and Context Map

Quarantine Sandbox Runtime is the canonical reusable isolation/evidence boundary. Core `sandbox_execution` owns backend-neutral isolation requirements and verified runtime state. Supporting `application_service` owns isolated service leases and bounded command execution. Supporting `artifact_analysis` owns artifact identity and analysis-evidence contracts. Podman/gVisor/containerd/Kubernetes/VM mechanics stay behind infrastructure ports/ACLs.

Wardnet retains gateway/SOC policy, maliciousness verdicts, incidents, quarantine/block/review, notification, and retention. `contextual-orchestrator` retains model/Agent orchestration, caller authorization, application selection, secrets, tasks, and user-visible actions. Consumers must use released versioned contracts/ACLs; sibling source, mutable branch dependencies, backend SDK leakage, and cross-service application-table SQL are forbidden.

Target bounded contexts remain Workload Admission, Isolation Policy, Runtime Provisioning, Network/Egress, Artifact Analysis, Evidence/Provenance, Session Lifecycle, and Recovery.

## Application-service isolation

| Capability | Current evidence | Maturity / next action |
| --- | --- | --- |
| Immutable workload identity | Digest-pinned lowercase SHA-256 OCI image references; no implicit pull. | Active-PR implementation; preserve through admission/release. |
| Rootless execution | Podman host state is inspected; non-rootless execution fails closed. | Exact-release-host proof required. |
| Filesystem/identity/namespace isolation | Read-only rootfs, bounded `noexec,nosuid,nodev` tmpfs, ignored image volumes, non-root UID/GID, isolated user/PID/IPC namespaces. | Maintain real-backend hostile regressions. |
| Capability/no-new-privileges | Container effective/bounding plus process effective/bounding/inheritable/permitted/ambient capability sets are required empty; no-new-privileges required. | Positive exact-head runtime proof required. |
| Seccomp / LSM | Host support alone is insufficient. Live process seccomp and AppArmor/SELinux security context are required. AppArmor `containers-default (enforce)` may match inspect `containers-default`; complain/bare/unconfined/empty/malformed/mismatched evidence fails closed. | Causal source repair is on #9; exact-head execution and positive LSM runner proof remain required. |
| Network / publication | Service profile uses per-sandbox internal DNS-disabled network and loopback-only publication; command profile uses `--network none`. | Controlled egress requires a separate reviewed profile. |
| Resource bounds | CPU/RAM/PID/tmpfs/lifetime/readiness/shutdown limits are validated and inspected. | Revalidate on supported cgroup/runtime profiles. |
| Caller ownership / idempotency | #6 scopes lease state by caller owner + request identity, rejects changed immutable reuse, and bounds cleanup work. | Authenticated transport and durable replay remain missing. |
| Cleanup | Post-create failures attempt cleanup and cleanup failure dominates when resource removal cannot be proven. | Exact-head execution remains required before GREEN. |
| Crash/restart recovery | No durable lease journal/orphan reconciliation. | Buyer gap: durable Session Lifecycle/Recovery plus crash/orphan E2E. |
| Stronger backends | No production gVisor/containerd/microVM adapter. | Add behind the same verified-isolation ACL only when justified. |

#9 exact head `693ee6c096b53f562fb106a3bcb9fb9efa092c8f` has CI `33731009971`: positive-LSM `100570709545`, verify `100570709834`, branch-coverage `100570709836`, coverage `100570709843`, and negative-rootless-AppArmor `100570709902` remain queued before checkout. Hosted admission is `.github#712`; reviewed positive-LSM capacity is `.github#1590`.

## Bounded command execution

PR #13 proposes the provider-neutral run-to-completion contract: `CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, and `execute_command`. Nonzero **workload** exit status is structured workload evidence, not a sandbox malfunction. The contract reuses digest/image, isolation-policy, and resource-bound types rather than creating a second isolation authority.

PR #14 proposes the production rootless-Podman backend and synchronous `quarantine-sandbox-runtime run` CLI. It uses `--network none`, bounded administrative Podman calls/output, live seccomp/LSM/capability attestation, `podman wait` for completion, retained bounded logs, and fail-closed cleanup.

The static-inspect fallback for fast-exiting commands remains rejected. `verify_command_isolation` requires live `podman top` process evidence; disappearance before attestation fails closed. Short-command support needs a reviewed start/hold/attest/release handshake or stronger backend primitive, not weaker static evidence.

### Current command-evidence RED

Current #14 contains `tests/podman_command_execution_wait_status_red.rs`. The fake Podman completes creation, start, and live isolation attestation, then its **administrative** `podman wait` process exits nonzero while printing `17`. That state must be `BackendCommandFailed { operation: "command_wait" }`; cleanup must still run, and `podman logs` must not be trusted afterward.

Current production `wait_for_command()` checks whether the bounded wait timed out but does not first reject a non-successful `podman wait` process status. Its stdout can therefore be promoted to `CommandExecutionResult.exit_code` even when the runtime failed to obtain trustworthy completion evidence. This is an evidence-integrity defect, not a workload failure. The test is intentionally RED-only until it can execute. The causal repair must also review the same boundary's post-kill timeout/status/truncation handling rather than weakening evidence or inventing a workload result.

Before this RED, command cleanup was already strengthened so malformed create output, start/isolation/log failures, and cleanup failure use one fail-closed precedence rule: if removal cannot be proven, `CleanupFailed` dominates the earlier backend error.

Exact #14 test-only head `579da1642725474083c4f8440164b55378a239ac` produced CI `33737594942`: hosted branch-coverage `100591738935`, coverage `100591738965`, verify `100591739035`, negative-rootless-AppArmor `100591739133`, and self-hosted positive-LSM `100591739050`. All are queued before checkout with `steps=[]` and no runner identity. The baseline commit advances the same branch, so the new exact head must acquire its own evidence; no predecessor result transfers.

## Artifact analysis

| Capability | Current state | Next buyer/security slice |
| --- | --- | --- |
| Immutable static evidence foundation | SHA-256 artifact identity, bounded source context, deterministic classification, ordered analyzer port, attributable failures on #1. | Integrate foundation and bind immutable release identity. |
| Verdict boundary | Runtime emits risk/analysis evidence; consumer retains business/security disposition. | Preserve; never promote risk score/verdict into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF adapters | Missing. | Add bounded adapters with tool/version/digest provenance. |
| Linux dynamic detonation | Missing. | Reuse verified sandbox or stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | Missing. | Separate Windows isolation pool preserving the common evidence contract. |
| Controlled network telemetry | Missing. | Add explicit sinkhole/approved-egress policy and bounded capture; no production credentials. |
| Durable chain of custody | Process-local only. | Add immutable storage/signing/retention/replay and recovery ownership before GA. |

## Release delivery

PR #10 makes the zero-release state executable: protected-source/tag identity, locked Cargo packaging, complete owned coverage gates, SPDX SBOM, SHA-256 manifest, provenance, reproducibility, rollback expectations, and immutable GitHub Release assets. It remains Draft and is not release authority.

Before `gh release create`, the release path must verify provenance for the exact `.crate` against `ContextualWisdomLab/quarantine-sandbox-runtime` and independently verify the SPDX predicate for the same package bytes. No external registry publication is currently authorized. The first distribution remains a GitHub Release only after one exact integrated protected source identity passes product/security/positive-LSM/coverage/package/SBOM/provenance/reproducibility/review/rollback gates together.

## Consumer compatibility and handoff

Production consumers must pin a future immutable released artifact/version plus checksum and verified provenance/SBOM. Open PR heads, branch names, semantic-version labels, unsigned tags, and transient Actions artifacts are not production dependencies.

| Consumer | Authority retained by consumer | Required handoff |
| --- | --- | --- |
| Wardnet | Gateway/SOC policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention. | Consume released artifact-analysis evidence only. |
| contextual-orchestrator | LLM/Agent orchestration, authorization, application selection, secrets, task/user actions. | Consume released application-service/command contract through ACL; never direct Podman/containerd or sibling source. |
| `.github` central review | Review verdict and executed-PoC policy. | After immutable command-runtime release, consume the released CLI/API contract rather than copying runtime policy. |

Wardnet and contextual-orchestrator currently have **0 releases** as well; no mutable consumer integration is release authority.

## Context Graph and Enterprise Architecture read-only dependency state

`ContextualWisdomLab/context-graph-contracts` remains the contract-only Shared Kernel and has **0 releases**. Quarantine-relevant open work includes release-source prerequisite #25 at `187f45927e697cfad9ac5b2523dfd86b695aa072` and Context Assertion/CloudEvent #21 at `b0c21f907a12b07a28cf38ed165ab6530855283e`. #25 has explicit `ubuntu-24.04` selectors but its current repository workflows remain queued before runner assignment; #21 remains Draft/non-mergeable with no immutable authority. No CGC PR head is production interoperability authority.

`ContextualWisdomLab/enterprise-architecture-core` also has **0 releases**. Draft #40 remains the quarantine/Context Fabric projection owner at `fadf27f0df1a865261c15dc64de2a9dc350e02d4`; its recorded quarantine producer snapshot predates the current #14 wait-evidence RED and is therefore read-only stale evidence, not authority. Allowed projection remains runtime/backend identity, technology/provider/version, lifecycle/risk context, ownership, remediation/transformation, and attestation provenance. Malware verdict/artifact risk score, foreign DB access, and source copying remain prohibited.

The dedicated Context Fabric writer owns CGC/EA source and PR-state repair. This quarantine writer supplies read-only producer evidence and acceptance criteria only.

## Highest-leverage buyer-visible gaps

1. Execute and causally repair the #14 administrative-wait evidence RED; do not promote failed `podman wait` metadata into workload evidence.
2. Restore hosted runner admission through `.github#712` and obtain actual exact-head execution instead of source churn.
3. Provide disposable positive-LSM runtime capacity through `.github#1590`, binding effective LSM/seccomp/capability/resource/cleanup evidence to one exact candidate.
4. Drain `#1 -> #6 -> #9 -> #10 -> #13 -> #14` by exact RED→causal GREEN→normal merge→non-force descendant restack, preserving unique deltas and reacquiring evidence after every movement.
5. Complete the first immutable release from one protected integrated source identity, then bump Wardnet/contextual-orchestrator/central-review consumers to that released contract.
6. Add durable authenticated admission/session/recovery/idempotency plus orphan reconciliation for remote/multi-process operation.
7. Add real artifact analyzers and dynamic detonation profiles with immutable chain-of-custody evidence rather than expanding consumer verdict authority into this runtime.

No item above is complete from documentation, queued workflows, local-only results, or open-PR artifacts alone.
