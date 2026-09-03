# Product and Technical Gap Baseline

Last reviewed on 2026-09-03 KST from fresh GitHub state and the current candidate stack. This file distinguishes protected/default-branch truth, active-PR implementation, test-only RED evidence, central control-plane dependencies, and immutable release authority.

## Repository and governance truth

- Default/integration branch: `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`.
- `main@6d27dba3d5b013b922e21757431330d048a91d70` remains the initialization commit and is not currently protected; it is not a release source.
- Organization ruleset `18156473` applies to `~DEFAULT_BRANCH` and requires one approving review, resolved review threads, non-fast-forward/deletion protection, and the central required workflow set.
- GitHub Releases: **0**. `0.1.0` is development metadata only.
- Queued, skipped, cancelled, absent, local-only, predecessor-head, static-only, or unassigned-runner results are non-passing evidence.

The current dependency-ordered Draft stack is:

```text
#1  runtime foundation                 @ d2893e39ef71d72d315f71f44ee04f498369b3e2
 -> #6  caller-scoped leases            @ eed831ded4da519d97df0d6e000682fe4b1c5d84
 -> #9  effective isolation             @ ecb31b5566c2acdecef5570257459e9cf52478ad
 -> #10 release evidence                @ 99f61823d1d6de6110a1c061f3ef3e9fddcc56ba
 -> #13 bounded command contract        @ 7b549b37e7516b3927920ec033146a1914b15fce
 -> #14 Podman command backend and CLI  @ 7c3e55ce0a33746927b944e815025f461eea5d79
```

Every child currently descends from the exact parent without force-updating history. ADR-0006, ADR-0007, and ADR-0008 remain **Proposed** while their decisions exist only on Draft/unmerged candidates and lack the required protected-integration/current-head evidence.

## Product responsibility and Context Map

Quarantine Sandbox Runtime is the canonical reusable isolation/evidence boundary. Core `sandbox_execution` owns backend-neutral isolation requirements and verified runtime state. Supporting `application_service` owns isolated service leases and bounded command execution. Supporting `artifact_analysis` owns artifact identity and analysis evidence contracts. Podman/gVisor/containerd/Kubernetes/VM mechanics remain infrastructure adapters behind those ports.

Wardnet retains gateway/SOC policy, maliciousness verdicts, incidents, quarantine/block/review, notification, and retention. `contextual-orchestrator` retains model/Agent orchestration, caller authorization, application selection, secrets, tasks, and user-visible actions. Consumers must use released versioned contracts/ACLs; sibling source, mutable branch dependencies, backend SDK leakage, and cross-service application-table SQL are forbidden.

Target bounded contexts remain Workload Admission, Isolation Policy, Runtime Provisioning, Network/Egress, Artifact Analysis, Evidence/Provenance, Session Lifecycle, and Recovery. The current code groups the first release slices under `sandbox_execution`, `application_service`, and `artifact_analysis`; further extraction must follow actual aggregate/transaction ownership rather than technical-directory fan-out.

## Application-service isolation

| Capability | Current evidence | Maturity / next action |
| --- | --- | --- |
| Immutable workload identity | Digest-pinned lowercase SHA-256 OCI image references; no implicit pull. | Active-PR implementation; preserve through admission and release. |
| Rootless execution | Podman host state is inspected; non-rootless execution fails closed. | Active-PR implementation; require exact-release-host proof. |
| Filesystem/identity/namespace isolation | Read-only rootfs, bounded `noexec,nosuid,nodev` tmpfs, ignored image volumes, non-root UID/GID, isolated user/PID/IPC namespaces. | Active-PR implementation; maintain real-backend hostile regressions. |
| Capability/no-new-privileges | Container effective/bounding sets plus process effective/bounding/inheritable/permitted/ambient sets are verified empty; no-new-privileges required. | Active-PR implementation; positive exact-head runtime proof required. |
| Seccomp / LSM | Host capability is insufficient by itself. Live process seccomp and AppArmor/SELinux label/domain evidence is required and ambiguous/unconfined/complain-mode evidence fails closed. | Active-PR implementation; positive LSM CI is owned through `.github#1590`. |
| Network / publication | Service profile uses per-sandbox internal DNS-disabled network and loopback-only publication; command profile uses `--network none`. | Active-PR implementation; controlled egress requires a separate reviewed profile. |
| Resource bounds | CPU/RAM/PID/tmpfs/lifetime/readiness/shutdown limits are validated and inspected. | Active-PR implementation; revalidate on exact supported cgroup/runtime profile. |
| Caller ownership / idempotency | #6 scopes lease state by caller owner + request identity, rejects changed immutable request/policy reuse, and bounds cleanup work. | Active-PR implementation; authenticated transport and durable replay remain missing. |
| Cleanup | Service and command paths attempt runtime-owned cleanup and fail closed when cleanup cannot be proven. | Two #14 cleanup-error-precedence RED candidates still require executable failure before production generalization. |
| Crash/restart recovery | No durable lease journal/orphan reconciliation. | Buyer-visible gap: add durable Session Lifecycle/Recovery contract and crash/orphan E2E after the foundation lands. |
| Stronger backends | No production gVisor/containerd/VM adapter. | Add only behind the same verified isolation ACL after the P0 contract stabilizes. |

### Exact current runtime/security evidence

Root #1 exact head `d2893e39...` has CI run `33721940096`; `branch-coverage`, `coverage`, `podman-e2e`, and `verify` materialized before checkout with no assigned runner and no executed steps. Fresh SAST, Security, Scorecard, and OSV runs on that same head are also queued. This is current admission evidence, not source GREEN.

Leaf #14 exact head `7c3e55ce...` has CI run `33723084105` with five materialized but unexecuted jobs: hosted `branch-coverage`, `coverage`, `verify`, `podman-e2e-negative-rootless-apparmor`, plus `podman-e2e-positive-lsm` on `[self-hosted, linux, cwl-hostile-workload, selinux]`. All currently have empty steps and no runner identity. `.github#712` owns the hosted/pre-checkout admission class; `.github#1590` owns positive LSM-capable capacity. Leaf source must not churn merely to retrigger either class.

## Bounded command execution

PR #13 proposes the provider-neutral run-to-completion contract: `CommandExecutionRequest`, `CommandExecutionResult`, `CommandExecutionBackend`, and `execute_command`. Nonzero workload exit status is structured workload evidence, not a sandbox malfunction. The contract reuses the existing digest/image, isolation-policy, and resource-bound types rather than creating a second isolation authority.

PR #14 proposes the production rootless-Podman backend and synchronous `quarantine-sandbox-runtime run` CLI. It uses the P0 isolation policy, `--network none`, bounded Podman invocation/output, `podman wait` for authoritative completion, and retained bounded logs.

A previously described static-inspect fallback for fast-exiting commands was rejected. `verify_command_isolation` now requires live `podman top` process evidence for seccomp, LSM, and all privilege-relevant capability sets; when the process exits before attestation, the current implementation fails closed. Commit `7c3e55ce...` repaired stale rustdoc that still described static configuration as a fallback. Future fast-command support requires a reviewed start/hold/attest/release handshake or equivalent stronger backend primitive, not weaker evidence.

Two test-only cleanup RED candidates remain on #14:

- `cleanup_failure_is_not_hidden_behind_container_logs_timeout`;
- `cleanup_failure_is_not_hidden_behind_effective_isolation_failure`.

They require a cleanup leak/failure to surface as `CleanupFailed` instead of being hidden behind the earlier logs/isolation error. Production cleanup behavior must not be generalized until those REDs actually execute and fail on the exact candidate.

## Artifact analysis

| Capability | Current state | Next buyer/security slice |
| --- | --- | --- |
| Immutable static evidence foundation | SHA-256 artifact identity, bounded source context, deterministic classification, ordered analyzer port, attributable failures on #1. | Integrate foundation and bind immutable release identity. |
| Verdict boundary | Runtime emits risk/analysis evidence; consumer retains business/security disposition. | Preserve; never promote risk score/verdict into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF adapters | Missing. | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux dynamic detonation | Missing. | Reuse verified sandbox or a stronger gVisor/microVM profile; never execute hostile bytes in host control process. |
| Windows detonation | Missing. | Separate Windows isolation pool preserving common evidence contract. |
| Controlled network telemetry | Missing. | Add explicit sinkhole/approved-egress policy and bounded capture; no production credentials. |
| Durable chain of custody | Process-local only. | Add immutable object storage/signing/retention/replay and recovery ownership before GA. |

## Release delivery

PR #10 turns the zero-release state into an executable release candidate: source-bound tag checks, locked Cargo packaging, exact 100% owned coverage gates, SPDX SBOM, SHA-256 manifest, provenance, reproducibility, rollback expectations, and immutable GitHub Release assets. It remains Draft and is not release authority.

The current release RED is `tests/release_attestation_verification_contract.rs`: before publication the workflow must consume and verify provenance for the exact `.crate` against `ContextualWisdomLab/quarantine-sandbox-runtime` and separately verify the SPDX 3 predicate for the same package bytes. Creating attestations without consuming/verifying both as a release gate is insufficient. Do not patch the workflow until the current RED executes and proves the failure.

No registry publication is currently authorized. The first approved distribution path remains a GitHub Release only after one exact integrated protected source identity passes product, security, positive-LSM, coverage, package, SBOM, provenance, reproducibility, review, and rollback gates. No release exists today.

## Consumer compatibility and handoff

Production consumers must pin a future immutable released artifact/version plus checksum and verified provenance/SBOM evidence. Open PR heads, branches, semantic-version labels, tags without provenance, and transient Actions artifacts are not acceptable production dependencies.

| Consumer | Authority retained by consumer | Required handoff |
| --- | --- | --- |
| Wardnet | Gateway/SOC policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention. | Consume released artifact-analysis evidence contract only. |
| contextual-orchestrator | LLM/Agent orchestration, authorization, application selection, secrets, task/user actions. | Consume released application-service/command contract through an ACL; never direct Podman/containerd or sibling source. |
| `.github` central review | Review verdict and what counts as executed PoC evidence. | After immutable command-runtime release, migrate its sandboxed verification path through the released CLI/API contract rather than copying runtime policy. |

## Context Graph and Enterprise Architecture read-only dependency state

`ContextualWisdomLab/context-graph-contracts` remains the contract-only Shared Kernel for canonical object/authority references, six-value truth/origin, bitemporal semantics, provenance, Context Assertion, CloudEvents, schema/conformance/admission, and package/release evidence. It has **no release**. Relevant current Draft state includes root #4 `03caa05e432a46227e16ecddd61ed825d1a104dd`, release-source prerequisite #25 `9283d7b8ed85b97b893eeacf339524177f3ffbfc`, DDD baseline #20 `475ce14185db697940e8219c3cda7f24d66f3ed7`, and Context Assertion/CloudEvent #21 `b0c21f907a12b07a28cf38ed165ab6530855283e`. The release/projection rebuild around those lanes remains `#19 -> #25 -> #20 -> #21`; queued or no-run evidence is non-passing. No open CGC head may be consumed as authoritative production interoperability.

`ContextualWisdomLab/enterprise-architecture-core` also has **no release**. Draft #40 current head `3734f0c8d2533015c2b2cbfe22d16f1a507881cb` explicitly models Quarantine Sandbox Runtime as an independently deployable foreign authority. Its allowed projection is limited to runtime/backend identity, technology/provider/version, lifecycle, architecture-risk context, ownership, remediation/transformation, and attestation provenance. It requires directional ACL interactions `contextual-orchestrator -> quarantine application-service lease` and `Wardnet -> quarantine artifact-analysis/evidence`; malware verdict/artifact risk score, foreign DB access, and source copying remain forbidden. #40 currently has no exact-head workflow runs and is not release authority.

The dedicated Context Fabric writer owns CGC/EA source and PR-state repair. This writer provides exact read-only evidence/acceptance criteria only.

## Highest-leverage buyer-visible gaps

1. Restore exact-head hosted runner admission through `.github#712`, then execute current REDs rather than mutating clean leaves to manufacture triggers.
2. Provide disposable positive-LSM runtime capacity through `.github#1590` and bind effective LSM/seccomp/capability/resource/cleanup evidence to the exact candidate.
3. Drain the dependency stack `#1 -> #6 -> #9 -> #10 -> #13 -> #14` by real RED→causal GREEN→normal merge→non-force descendant restack, preserving unique deltas and reacquiring evidence after every parent movement.
4. Complete the first immutable release from one protected integrated source identity and then bump Wardnet/contextual-orchestrator/central-review consumers to the released contract rather than PR heads.
5. Add durable authenticated admission/session/recovery/idempotency and orphan reconciliation for multi-process/remote operation.
6. Add real artifact analyzers and dynamic detonation profiles with immutable evidence/chain-of-custody instead of expanding consumer verdict authority into this runtime.

No item above is considered complete from documentation, queued workflows, local-only results, or open-PR artifacts alone.
