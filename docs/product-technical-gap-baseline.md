# Product and Technical Gap Baseline

Last reviewed on 2026-09-01 from live GitHub state. The repository default/integration branch is `develop`; its current tip was `60a85c7633e03b425b67159ec6822c8178cf87ea` at this review and organization ruleset `18156473` applies to the default branch. Stable `main` remained at initialization commit `6d27dba3d5b013b922e21757431330d048a91d70` and was not protected. No product release exists. The package version is `0.1.0`, but that version is still development metadata rather than a released artifact identity.

The active product stack is dependency ordered:

```text
#1 runtime foundation @ c78fd491f84fae773b3691b10b6a0c21940808d5
→ #6 caller-scoped leases @ a7d7ca0605da7f0f07dedf6e77df86b6850c7b07
→ #9 effective isolation @ 3ec25392f829a5cbe0a1df4c632f299bb4d7d3a0
→ #10 release delivery evidence
```

PR #1 is now behind two default-branch bookkeeping commits and must be reconciled non-destructively before protected integration. PRs #1, #6, #9, and #10 remain Draft. Queued, skipped, cancelled, stale, predecessor-head, or unassigned-runner results are non-passing evidence.

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

Core `sandbox_execution` owns backend-neutral isolation requirements and verified runtime state. `application_service` currently owns the process-local lease facade and caller-scoped idempotency while durable admission/session/recovery contracts are extracted. Infrastructure adapters own Podman/gVisor/containerd implementation details. Wardnet retains gateway/SOC policy, maliciousness verdict, incident, quarantine/block/review, notification, and retention authority. `contextual-orchestrator` retains LLM/Agent orchestration, caller authorization, application selection, secrets, and user-visible actions.

## Application-service isolation

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Immutable workload identity | Request accepts digest-pinned lower-case SHA-256 OCI references; Podman launch is no-pull. | Implemented | Preserve digest identity through admission and release. |
| Rootless backend | Podman host state is inspected and non-rootless execution fails closed. | Implemented | Require the same positive proof on every release head. |
| Filesystem isolation | Read-only rootfs, bounded noexec/nosuid/nodev tmpfs, image volumes ignored, no arbitrary host mounts. | Implemented | Add only typed reviewed input mounts if a buyer flow requires them. |
| Privilege isolation | Effective and bounding capability sets must both be empty; no-new-privileges, non-privileged mode, numeric non-root identity, user/PID/IPC namespace checks are inspected after start. | Implemented on #9 | Keep hostile process-boundary regressions and real release proof. |
| Seccomp | Host seccomp must be enabled with a concrete profile; `seccomp=unconfined` fails closed. | Implemented on #9 | Prove the effective release runner profile. |
| LSM | AppArmor or SELinux host capability plus effective container label/profile is required. | Product logic correct; hosted positive proof unavailable | `.github#1590` must provide disposable LSM-capable release/security infrastructure or a separately reviewed stronger backend. Do not downgrade. |
| Network isolation | Per-sandbox internal DNS-disabled network; external egress denied; service publication must parse as IPv4 loopback only. | Implemented | Controlled egress must be a separate versioned profile. |
| Credential isolation | No consumer/provider credentials, arbitrary environment, runtime socket, host devices, or ambient proxy inheritance. | Implemented | Future secrets require a task-scoped broker and explicit authorization contract. |
| Resource limits | Memory, CPU, PID, lease duration, tmpfs, readiness and shutdown bounds are validated and inspected. | Implemented on #9 | Re-run on the exact supported release host/cgroup profile. |
| Runtime identity | Lease schema `1.2.0` carries `backend_id`, inspected `backend_version`, full policy SHA-256 and effective control statuses. | Implemented on #9 | Bind released source/package identity and durable signed evidence. |
| Caller ownership | `LeaseOwnerId` is command context; wrong-owner termination fails before backend cleanup. | Implemented on #6 | Bind owner identity to an authenticated transport before remote/multi-process use. |
| Idempotent launch | Process-local coordinator keys by owner + request, rejects changed request/policy reuse and concurrent duplicate launch. | Implemented on #6 | Durable replay/admission contract remains missing. |
| Expiry cleanup fairness | Bounded 64-lease cleanup cannot let repeatedly failing early entries starve later expired leases. | Implemented on #6 | Preserve after recovery state becomes durable. |
| Crash/restart orphan reclamation | No durable lease journal or orphan reconciliation. | Missing | Extract `recovery` and `session_lifecycle`; add deterministic crash/orphan E2E. |
| gVisor/containerd/VM backends | No production adapter. | Missing | Add only behind the same verified isolation ACL after P0 contract/release stabilizes. |

## Bounded command execution

`ContextualWisdomLab/.github` central review (Noema, OpenCode Review) currently evaluates every PR by
reading diffs and reasoning about them; it never executes the code under review. Its own
`scripts/ci/sandboxed_verify.py` and `scripts/ci/sandboxed_web_e2e.py` -- named as the
"actually-executed PoC" evidence mechanism the OpenCode review gate requires -- isolate locally on the
CI runner itself (a scrubbed `tempfile` workspace plus direct `subprocess.run`, and `bwrap` when
available) rather than calling out to this runtime. This is a third consumer path, alongside Wardnet
(issue #38) and `contextual-orchestrator` (issue #991), not previously tracked in this table.

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Bounded command-to-completion contract | `CommandExecutionRequest`/`CommandExecutionResult`/`CommandExecutionBackend` (ADR-0007) validated, coordinated by `execute_command`. | Implemented on active PR | Merge through the existing dependency-ordered stack once ready; contract is now consumed by a real backend (below), so treat it as stable. |
| Real command-execution backend | `RootlessPodmanAdapter::run_command_at` (ADR-0008, `src/infrastructure/podman.rs`) creates a fresh `--network none`, read-only-rootfs, all-capabilities-dropped, non-root sandbox per call, verifies the same P0 controls as the service lease (falling back to Podman's static container-configuration record when a fast-exiting command's live process can no longer be observed), and observes completion with `podman wait`/`podman logs`. Proven against a real rootless Podman installation (locally, Podman 6.1.0; three genuine cross-Podman-version deserialization/comparison gaps were found and fixed this way -- see ADR-0008) via `tests/podman_command_execution_e2e.rs`, `#[ignore]`d pending `.github#1590` the same way the service lease's own real-isolation tests are. | Implemented on active PR, real-isolation proven locally, not yet CI-verified | Land `.github#1590`'s dedicated LSM-capable runner so `tests/podman_command_execution_e2e.rs` runs as CI evidence, the same blocker the service lease already carries; re-run against Podman 5.8.4 (CI's pin) to confirm the three version-specific fixes hold there too. |
| CLI entrypoint | `quarantine-sandbox-runtime run --image <digest> [resource flags] -- <command> [args...]` (`src/main.rs`, `[[bin]]` in `Cargo.toml`, ADR-0008): validates against a hardcoded operator `IsolationPolicy` ceiling, prints `CommandExecutionResult` as JSON on stdout, and exits with the sandboxed command's own exit code. Smoke-tested directly against real Podman (exit-code passthrough for success, nonzero, and a validation failure). No HTTP transport (not needed for this contract's one-shot shape; see ADR-0008) and no config file (the policy ceiling is not yet operator-configurable). | Implemented on active PR | None to reach parity with a shell-out caller. Add a config-file-loaded policy only if an operator actually needs a ceiling other than the hardcoded default. |
| `.github` central-review wiring | `scripts/ci/sandboxed_verify.py`/`scripts/ci/sandboxed_web_e2e.py` are unmodified; they do not call this runtime. | Missing (external repo) | Owned by `.github`; the backend and CLI this row depended on now exist. File the owner-path issue and wire `sandboxed_verify.py`/`sandboxed_web_e2e.py` to shell out to the CLI (or its eventual released binary) through an ACL. Track as a gap in `.github`'s own `docs/product-technical-gap-baseline.md`. |

### Current real-backend blocker

PR #9 exact head `3ec25392f829a5cbe0a1df4c632f299bb4d7d3a0` closes the residual bounding-capability defect found by hostile RED `55704a220eceb25e6f7f586b0f102c5bb6028e98`. Real Podman CI on that exact head uses Podman 5.8.4 in rootless mode and fails closed at `IsolationVerificationFailed { control_name: "lsm" }`; the unconditional final container/network leak check succeeds. This is evidence that the Ubuntu-hosted lane cannot prove the P0 LSM requirement, not evidence that the requirement should be relaxed. The bounded command-execution backend (ADR-0008) carries the identical `.github#1590` blocker for CI-verified real-isolation evidence; its own real-Podman proof so far is local-only (see the row above).

Central issue `ContextualWisdomLab/.github#1590` owns the external runner/security prerequisite. Preferred first GREEN profile is a disposable SELinux-capable Linux runner with rootless Podman, seccomp, effective process label, empty effective/bounding capability sets, requested cgroup limits, deny-by-default egress, loopback-only service publication, and complete cleanup on the unchanged release candidate SHA.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | Immutable SHA-256 identity, bounded source context, deterministic format classification, ordered analyzer port and attributable failures. | Implemented on #1 | Integrate and retain exact release evidence. |
| Verdict boundary | `EvidenceBundle` and disposition describe risk/analysis evidence; consumer verdict remains required. | Implemented | Never promote evidence score/verdict into foreign authoritative truth. |
| YARA-X / capa / Ghidra / LIEF | No production adapters. | Missing | Add one bounded adapter per TDD slice with tool/version/digest provenance. |
| Linux dynamic detonation | No artifact detonation vertical yet. | Missing | Reuse the verified sandbox boundary or a stronger gVisor/microVM profile; never execute hostile bytes in the host control process. |
| Windows detonation | No production pool. | Missing | Separate Windows isolation boundary preserving common evidence contracts. |
| Controlled network telemetry | No sinkhole/approved-egress analysis profile. | Missing | Add explicit policy, bounded capture and no production credentials. |
| Durable evidence/chain of custody | Evidence is process-local. | Missing | Add immutable object storage, retention/signing/replay and recovery ownership before GA. |

## Release delivery

The absence of a release is an implementation gap, not a reporting state. Stacked PR #10 creates the first executable release-delivery contract while remaining blocked behind the product/security stack.

Current release facts:

| Item | Current state |
| --- | --- |
| Package version | `0.1.0` development metadata |
| GitHub Releases | none |
| Stable release source | no integrated protected `main` product commit |
| Application-service lease contract | `1.2.0` |
| Artifact-analysis request/evidence contract | `1.0.0` |
| Release evidence contract | `1.0.0` candidate on #10 |
| Cargo source package | candidate workflow only; no released bytes |
| SPDX SBOM | candidate workflow only |
| SHA-256 manifest | candidate workflow only |
| GitHub provenance/SBOM attestation | candidate workflow only |
| Byte reproducibility | candidate workflow performs two clean `cargo package --locked` builds before release |
| Positive hostile-runtime release proof | externally blocked by missing LSM-capable runner `.github#1590` |

The release workflow is tag-driven and must fail unless `vX.Y.Z` matches `Cargo.toml`, a dated `CHANGELOG.md` section exists, the tag SHA equals the live protected `main` tip, complete verification/coverage succeeds, and the dedicated LSM-capable real runtime E2E passes on the same SHA. Packaging occurs only after those gates. GitHub Release assets are the Cargo package, SPDX 3 SBOM, `SHA256SUMS`, and `release-evidence.json`, with GitHub build-provenance and SBOM attestations bound to the package bytes.

No crates.io or other registry publication is currently authorized/configured by this repository. The first distribution channel is therefore GitHub Release after the protected release gate; a registry adapter requires an explicit credential/namespace/rollback ownership decision.

## Consumer compatibility and handoff

`docs/contracts/consumer-contract.md` is the pre-release compatibility baseline. Strict consumers validate an exact schema version they support and do not infer compatibility from the repository package version. Application-service lease `1.2.0` intentionally adds required security evidence compared with `1.1.0`; old strict consumers must upgrade explicitly.

Production consumers must pin a released package plus SHA-256 and verified provenance/SBOM evidence. A PR head, branch, semantic-version label, tag alone, or transient Actions artifact is insufficient.

| Consumer | Authority retained by consumer | Integration state |
| --- | --- | --- |
| Wardnet | SOC/gateway policy, maliciousness verdict, incidents, quarantine/block/review, notification, retention | Issue #38 remains the owner path; must consume released artifact-analysis evidence contract only. |
| contextual-orchestrator | model/Agent orchestration, authorization, application selection, secrets, task/user actions | Issue #991 remains the owner path; must consume released application-service lease contract through an ACL, never direct Podman/containerd or sibling source. |
| `.github` central review (Noema, OpenCode Review) | PR review verdict, evidence-gate policy, what counts as an "actually-executed PoC" | No owner-path issue filed yet. Currently isolates locally (`sandboxed_verify.py`/`sandboxed_web_e2e.py`); must consume a released command-execution result contract through an ACL once a real backend and entrypoint exist -- see "Bounded command execution" above. |

## Context Graph and Enterprise Architecture read-only integration

`ContextualWisdomLab/context-graph-contracts` is the contract-only Shared Kernel for canonical authority/object references, truth status/origin, bitemporal semantics, provenance, Context Assertion, CloudEvents and conformance/admission. `ContextualWisdomLab/enterprise-architecture-core` owns authoritative EA decisions. This quarantine writer does not mutate either repository while the Context Fabric writer is active.

Fresh read-only state at this review:

- Context Graph open stack remains `#4 → #6 → #7 → #8 → #12 → #13 → #14 → #16 → #17 → #18 → #19 → #20 → #21`. PR #21 exact head `a3a3125619ed6e777818811b1c0b97f3a4574b73` carries structured Context Assertion CloudEvent semantics but remains Draft/unreleased; release/provenance lanes are not complete.
- EA Core PR #40 exact head `bd91a87e4cc45f2b205f410968b75b151a92bc4c` now explicitly models Quarantine Sandbox Runtime as an independently deployable foreign authority. Its validator requires `direction_code=inbound_projection`, `exchange_kind=context_assertion_cloudevent`, and `ea_core_owns=false`; malware verdict/risk score remain forbidden authoritative EA facts. #40 remains Draft and current hosted lanes are runner-unassigned/non-passing.
- Both Context Fabric repositories still have branch-governance/release dependencies in central `.github` owner paths. No unreleased Context Graph PR head may be treated as the production interoperability dependency.

After both required releases exist, quarantine runtime/backend identity, technology/provider/version, lifecycle, architecture-risk context, ownership, remediation/transformation and attestation provenance may flow through the released Context Assertion/CloudEvent contract. Malware verdicts and artifact risk scores remain risk evidence and are not copied into EA authoritative facts. Cross-service application-table SQL remains forbidden.

## Verification and governance gaps

- Organization runner queue starvation remains tracked by `.github#712`; clean leaf heads are not churned merely to retrigger unassigned jobs.
- The default-branch organization ruleset requires a pull request, one approval, resolved review threads and central required workflows. Administrator bypass capability is not routine integration evidence.
- PR #1 has no qualifying approval and its exact CI/Security/SAST runs remain queued at the latest read.
- PR #1 must be reconciled with the two live `develop` bookkeeping commits before integration; descendant evidence is regenerated after any ancestry change.
- `main` is not yet a protected product release branch in this repository. The release workflow itself rejects an unprotected `main`, but repository governance must establish the protected stable path before any tag.
- The LSM-capable hostile-workload release runner is an actual external security/infrastructure dependency owned by `.github#1590`.

## Next bounded slices

1. Reconcile #1 non-destructively with current protected `develop`, then regenerate exact-head product/security/review evidence.
2. Integrate dependency order #1 → #6 → #9 → #10 only through then-live protection; no predecessor evidence transfer.
3. Provision and validate `.github#1590` so the exact release candidate obtains positive LSM-capable real-backend proof.
4. Promote `0.1.0` from development metadata to a dated changelog release only after the integrated stable source, consumer schemas, release evidence, review and security gates all agree; then create the immutable GitHub Release.
5. Update Wardnet #38 and contextual-orchestrator #991 to pin the released artifact SHA-256/provenance and exact schema versions.
6. Extract durable Workload Admission, Session Lifecycle and Recovery contracts; add crash/replay/resource-reservation E2E.
7. Add artifact-analysis adapters and dynamic detonation only on top of the released isolation boundary.
8. Continue Context Fabric linkage read-only until released shared contracts exist; then hand off exact runtime release evidence to its sole writer.
9. Land `.github#1590`'s dedicated LSM-capable runner so `tests/podman_command_execution_e2e.rs` (ADR-0008's real-Podman-backed `CommandExecutionBackend` and `quarantine-sandbox-runtime run` CLI, already implemented and locally proven) runs as CI evidence, then file the owner-path issue for `.github` central review and wire `scripts/ci/sandboxed_verify.py`/`scripts/ci/sandboxed_web_e2e.py` to shell out to the CLI (or its eventual released binary) through an ACL.
