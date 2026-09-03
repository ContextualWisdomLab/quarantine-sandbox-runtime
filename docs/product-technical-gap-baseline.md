# Product and Technical Gap Baseline

Last reviewed on 2026-09-03 KST from fresh GitHub state and the current candidate stack. This ledger separates protected/default-branch truth, active-PR implementation, executable evidence, central control-plane dependencies, and immutable release authority. Workflow/run identifiers are evidence snapshots, not durable architecture authority.

## Repository and governance truth

- Default/integration branch remains `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` at this review point.
- GitHub Releases: **0**. Cargo version `0.1.0` is development metadata, not a shipped release.
- Organization ruleset `18156473` follows `~DEFAULT_BRANCH`; it currently requires one approving review, resolved review threads, nine central required workflows, and non-fast-forward/deletion protection. Branch/review governance must be reread before every merge.
- Queued, skipped, cancelled, absent, predecessor-head, static-only, local-only, or unassigned-runner results are non-passing evidence.

Current dependency order is:

```text
#1  runtime foundation                 @ d2893e39ef71d72d315f71f44ee04f498369b3e2
 -> #6  caller-scoped leases            @ eed831ded4da519d97df0d6e000682fe4b1c5d84
 -> #9  effective isolation             @ 693ee6c096b53f562fb106a3bcb9fb9efa092c8f
 -> #10 release evidence                @ fce2413c9d32bf0363bb38ff7669d6f4ee312738
 -> #13 bounded command contract        @ db5433781be679e7a4239f88b9c435ad7bd7f64c
 -> #14 Podman command backend and CLI  @ d9c8aa94f1edfd9576b7d9f3e05c65b3127c9898
```

The stack remains Draft and dependency-ordered. #14 non-force extends predecessor `964f2a5c0d6770df294edb3d6c8c820e77355ac0`; its intervening delta strengthens the partial-create cleanup regression and refreshes this ledger. No predecessor check/review evidence transfers across head/base movement. ADR-0006, ADR-0007, and ADR-0008 remain **Proposed** while unmerged and without protected-integration/current-head acceptance.

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

#9 exact head `693ee6c096b53f562fb106a3bcb9fb9efa092c8f` has CI `33731009971`. Its positive-LSM job `100570709545` requests `[self-hosted, linux, cwl-hostile-workload, selinux]`; verify/coverage/branch-coverage/negative-rootless-AppArmor use `ubuntu-24.04`. All five remain pre-checkout with `runner_id=0` and `steps=[]`. Hosted admission is owned by `.github#712`; reviewed positive-LSM capacity is owned by `.github#1590`.

## Bounded command execution

PR #13 proposes the provider-neutral run-to-completion contract: `CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, and `execute_command`. Nonzero **workload** exit status is structured workload evidence, not a sandbox malfunction. The contract reuses digest/image, isolation-policy, and resource-bound types rather than creating a second isolation authority.

PR #14 proposes the production rootless-Podman backend and synchronous `quarantine-sandbox-runtime run` CLI. It uses `--network none`, bounded administrative Podman calls/output, live seccomp/LSM/capability attestation, `podman wait` for completion, retained bounded logs, and fail-closed cleanup.

The static-inspect fallback for fast-exiting commands remains rejected. `verify_command_isolation` requires live `podman top` process evidence; disappearance before attestation fails closed. Short-command support needs a reviewed start/hold/attest/release handshake or stronger backend primitive, not weaker static evidence.

### Current command-evidence repairs

Current #14 source lineage contains three causal repairs with focused regression fixtures. They are repair candidates, not GREEN, until exact-head execution actually runs.

1. `tests/podman_command_execution_wait_status_red.rs` covers a successful sandbox whose **administrative** `podman wait` process exits nonzero while printing a plausible workload status. Production now requires a successful wait wrapper, rejects administrative output truncation, and does not call `podman logs` after untrustworthy wait evidence. The post-kill wait uses the same bounded checked-output failure contract.
2. `tests/podman_command_execution_wait_parse_red.rs` covers successful wrapper execution with malformed wait stdout. `parse_wait_exit_code` no longer fabricates `-1`; malformed or missing text is `MalformedIsolationInspection`, with ordinary and post-kill operation identity kept distinct.
3. `tests/podman_command_execution_create_failure_cleanup_red.rs` now covers both a `podman create` process that may persist the deterministic sandbox name before failing and cleanup failure on that path. Production attempts idempotent cleanup with `podman rm --force --ignore <sandbox>` even when create fails. Absence and successful removal are leak-free terminal states; cleanup failure surfaces `CleanupFailed`; a failed create must never advance to `podman start`.

Existing cleanup regressions also cover malformed create output, start/isolation/log failures, log timeout/status failure, and cleanup-error precedence. A runtime administrative failure must never be recast as workload output.

Current #14 exact head `d9c8aa94f1edfd9576b7d9f3e05c65b3127c9898` has CI `33749366734`: verify `100629135140`, branch-coverage `100629135147`, negative-rootless-AppArmor `100629135208`, coverage `100629135307`, and positive-LSM `100629135461`. Every job is still queued before checkout with `steps=[]`, no runner identity, and therefore no executable source evidence. The positive-LSM job requests `[self-hosted, linux, cwl-hostile-workload, selinux]`; the remaining jobs request `ubuntu-24.04`.

Predecessor CI `33749051365` on `964f2a5c...` completed as **cancelled** with all five jobs still `runner_id=0` and `steps=[]`. It is historical only and cannot transfer GREEN to `d9c8aa94...`. The central owner paths `.github#712` and `.github#1590` have fresh exact quarantine specimens; leaf source churn and gate weakening are prohibited.

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

No mutable consumer integration is release authority.

## Context Graph and Enterprise Architecture read-only dependency rule

`ContextualWisdomLab/context-graph-contracts` remains the contract-only Shared Kernel and `ContextualWisdomLab/enterprise-architecture-core` remains the EA Decision Plane. Their dedicated Context Fabric writer owns source and PR-state changes. This quarantine writer may inventory live state and provide exact producer evidence/RED-GREEN acceptance only; it must not write those repositories.

Quarantine may project runtime/backend identity, technology/provider/version, lifecycle/risk context, ownership, remediation/transformation, and attestation provenance only through a future released compatible Context Graph contract. Malware verdict/artifact risk score, foreign DB access, source copying, and mutable-branch production dependencies remain prohibited.

## Highest-leverage buyer-visible gaps

1. Restore hosted runner admission through `.github#712` and execute current exact-head repository gates instead of treating pre-checkout queueing as GREEN.
2. Provide disposable positive-LSM runtime capacity through `.github#1590`, binding effective LSM/seccomp/capability/resource/cleanup evidence to one exact candidate.
3. Execute the #14 wait-evidence and partial-create cleanup regressions against the current exact source lineage; repair any new causal failure rather than weakening the evidence contract.
4. Drain `#1 -> #6 -> #9 -> #10 -> #13 -> #14` by exact RED→causal GREEN→normal merge→non-force descendant restack, preserving unique deltas and reacquiring evidence after every movement.
5. Complete the first immutable release from one protected integrated source identity, then bump Wardnet/contextual-orchestrator/central-review consumers to that released contract.
6. Add durable authenticated admission/session/recovery/idempotency plus orphan reconciliation for remote/multi-process operation.
7. Add real artifact analyzers and dynamic detonation profiles with immutable chain-of-custody evidence rather than expanding consumer verdict authority into this runtime.

No item above is complete from documentation, queued/cancelled workflows, local-only results, or open-PR artifacts alone.
