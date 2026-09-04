# Product and Technical Gap Baseline

Last reviewed on 2026-09-05 KST against dependency-root PR #1 exact docs-only head `83115eda7c20c520d039192d253e4b4de0fe3b9d`, latest root test-bearing head `c43e5ca27acd96d085a4719fe5cb69de270aa723`, command-runtime RED head `585f3d955bddfb95f28e5918cfcadcac632589df`, and protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. Predecessor evidence never transfers to a moved head.

## Product responsibility

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, command/artifact-analysis isolation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Agent/chat consumers retain task/tool/application authorization, identity, secrets, and user-action authority.

## Current P0 isolation gaps

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Effective LSM | Hosted Ubuntu has proved fail-closed behavior when positive LSM confinement is unavailable; dedicated positive lane remains `[self-hosted, linux, cwl-hostile-workload, selinux]`. | Negative evidence exists; positive release proof pending | Keep hosted negative lane and #1590 positive lane distinct. |
| Application resource enforcement | Draft #19 `03c76208c97f302551c9e40b71fcc8e559c04cc3` preserves tmpfs/timeout/cgroup RED. | RED staged | Execute on stabilized ancestry, then applied-config GREEN and live enforcement proof. |
| Application runtime identity | Draft #21 `787d3b5e23d81886cb3d6794856b0f1763c2fe93` preserves same-request/same-second collision RED. | RED staged | Execute before adding collision-resistant invocation identity. |
| Application network attachment | Draft #23 `9b6ea9fe2e160c445ae7d9c8b3d7f89ef2870ddb` preserves exact attachment/additional-network RED. | RED staged | Execute before binding effective attachment proof. |
| Command pre-attestation execution | Issue #25 and `tests/podman_command_execution_pre_attestation_red.rs` show the command path currently binds the consumer command, calls `podman start`, then performs live `top`/LSM/seccomp/capability attestation. | P0 RED checked in at `585f3d955bddfb95f28e5918cfcadcac632589df`; production unchanged | RED must fail because consumer payload becomes runnable before attestation. GREEN requires trusted hold/attest/release or equivalent backend primitive; static-inspect reordering is insufficient. |

## Command-execution isolation

Draft #13 current `cbfe4b2eb5f90120782348fb8e5000c2ba9c15a3` owns the provider-neutral command contract. Draft #14 current ancestry preserves Podman command backend/CLI, Proposed ADR-0008, per-stream bounded completion, invocation-unique `qsr-cmd-*` identity, Podman 4.9-compatible create argv, and exact-revision source staging. Historical TDD debt for the cancelled identity/Podman-4.9 RED and later supervision increments remains explicit; predecessor checks do not transfer.

Configured launch intent, backend-applied inspection, live effective enforcement, and pre-execution authorization are distinct evidence levels. A positive attestation gathered only after a hostile consumer payload became runnable does not prove the pre-execution security boundary.

## Artifact analysis

| Capability | Current evidence | Status | Required action |
| --- | --- | --- | --- |
| Static foundation | SHA-256 identity, bounded ingestion, format detection, analyzer interface, deterministic evidence, and failure attribution exist on active foundation. | Active PR | Preserve under `artifact_analysis`; complete exact-head coverage/review. |
| Claude plugin package quarantine profile | Draft #18 preserves issue #17's RED for immutable catalog/source/artifact identity, fixed probe codes, bounded resources, credential-free/default-deny execution metadata, raw duplicate-JSON rejection, marketplace blob identity, and canonical repository-relative paths. Production `AnalysisRequest` still cannot represent the positive profile. | RED retained during non-force adoption of current #14 | Execute on current ancestry and require the positive profile case to fail for the missing public contract before implementation. RFC 7493/RFC 8259 constraints remain in doctoring traceability. |
| YARA-X / capa / Ghidra / LIEF | No production adapters yet. | Missing | Add one evidence producer per bounded increment with exact version/digest provenance. |
| Dynamic Linux/Windows detonation | No release-grade detonation worker exists. | Missing | Consume sandbox Core/stronger backend; never execute hostile bytes in control process. |
| Durable evidence/signing | In-memory evidence only. | Missing | Add persistence only after owner/retention/recovery/3NF/idempotency ADR. |

## Standards and doctoring traceability

Podman `create` prepares a container for its specified command without starting it; `podman start` starts the created container. NIST SP 800-190 treats container runtime/isolation as a security concern. Issue #25 maps these semantics to `run_command_at` and the payload-side-effect RED. Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Artifact-analysis duplicate-member behavior is grounded in RFC 7493 §2.3 and RFC 8259 §4; both are retained in `docs/doctoring/REFERENCES.md` and `STANDARD_TRACEABILITY.md`.

## Verification and release state

- Protected/default `develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea`, protected.
- Root PR #1 exact `83115eda7c20c520d039192d253e4b4de0fe3b9d` remains Draft; its causal issue #24 RED is latest test-bearing `c43e5ca27acd96d085a4719fe5cb69de270aa723`.
- Root CI `33905792562` remains pre-checkout queued: hosted jobs `101130380106`, `101130380122`, `101130380124`, `101130380157`; positive-LSM `101130379895` is separately queued.
- Live dependency chain is #6 `63ce9b1074bc3921c39eca26df53f7714e9cf392` -> #9 `bbb5efa95640a950dd9a53264034ea51fec00d41` -> #10 `65d02ec10a29c60499fb8fca6e1d0a5db63f145b` -> #13 `cbfe4b2eb5f90120782348fb8e5000c2ba9c15a3` -> #14 current parent. #18 is being non-force adopted above current #14 because its previous base was stale/non-mergeable.
- #14 causal test-bearing `585f3d955bddfb95f28e5918cfcadcac632589df` materialized CI `33909801395`, but all hosted and positive-LSM jobs remain pre-checkout queued; the #25 RED has not executed.
- Organization ruleset `18156473` is active: one approval, stale-review dismissal, review-thread resolution, seven central required workflows, deletion prohibition, non-fast-forward prohibition. Admin bypass is not merge evidence.
- GitHub Releases is empty; no immutable consumer authority exists.

## Protected integration and release authority

PR-head GREEN is necessary but insufficient. A normal merge can create a different protected integration SHA, so release authority requires native CI/security/runtime evidence on the exact protected `develop` integration head. Issue #24 treats a missing post-integration CI run as release-evidence failure. Draft #10 separately keeps the protected-source RED so release preflight cannot hard-code stale `main` assumptions.

Consumers may use only a future immutable released runtime artifact and versioned contract/ACL. Direct Podman/containerd calls, sibling source imports, mutable PR heads, and cross-service SQL are not integration mechanisms.

The first release requires complete owned statement/branch/edge coverage and public rustdoc, realistic rootless isolation E2E, positive effective LSM/seccomp/capability/resource/network/cleanup evidence, pre-execution command payload isolation proof, required review/security/SAST gates, package smoke, SPDX SBOM, provenance, reproducibility, upgrade/rollback evidence, and immutable artifact identity.

## Next bounded slices

1. Execute issue #25 RED on current command ancestry; only after causal failure add the minimal two-phase hold/attest/release GREEN.
2. Execute root issue #24 RED for native protected-`develop` CI before changing the stale push trigger.
3. Reacquire exact-head evidence for #6 -> #9 -> #10 -> #13 -> #14; no predecessor evidence transfers.
4. Execute #19 resource, #23 network-binding and #21 runtime-identity REDs after root stabilization, then apply only causal GREENs.
5. Execute #18 artifact-analysis contract RED on the newly adopted current #14 ancestry; no production profile implementation before causal failure.
6. Publish the first immutable runtime release only from one exact integrated protected head, then hand off released version/digest pinning to consumers.
