# Product and Technical Gap Baseline

Last reviewed on 2026-09-19 KST against protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea` and the live owner PR refs below. This file is the repository-wide single-writer current baseline. Predecessor checks never transfer to a moved head, mutable PR heads are not consumer authority, and GitHub `mergeable=true` is not release readiness.

The full historical causal ledger is preserved in [`product-technical-gap-history-2026-09-16.md`](product-technical-gap-history-2026-09-16.md). Historical present-tense statements there are evidence, not current authority.

## Integration root and release authority

Canonical integration root #1 remains exact `a85dc86c00f00354d9ddb9bf7c291c2c1cd40884`, Draft/open, targeting protected `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. Native CI `35180762143` has GREEN verify, production coverage, branch coverage, and hosted negative rootless/AppArmor jobs; positive SELinux received no eligible runner and was cancelled. Central Dependency Review `.github#810` and CodeQL terminal-publication `.github#1929` remain release blockers. GitHub Release inventory remains empty. Hosted sibling GREEN does not substitute for positive effective-LSM, central security terminal evidence, protected integration, or immutable publication.

## Current owner authority

### Command runtime and repository fitness

Command/runtime owner #14 remains exact `73a191c65cfe1dd16447921acc8f96b306583439`. Its hosted verify and negative-rootless jobs are GREEN, while coverage admission remains below repository-wide 100% and positive SELinux has no qualifying execution.

Repository-fitness descendant #112 is exact `8991d4a22004d07077db78e57e125088fcd16de4`. Predecessor `48ad86148be70fff165800d07221409bc7521af5` executed the physical-branch/codegen RED: complementary true/false execution across duplicate LLVM codegen records for one physical branch was still reported as `1/2`. Semantic `89c182151051668f4f468c53e64a947d965dda95` now unions outcomes by physical source identity, preserves two outcomes per physical branch, fails closed on denominator mismatch, and uses the same physical counts in diagnostics. Current CI `35379471155` remains current-head authority; predecessor success never transfers.

Independent process-lifecycle owner #125 is exact `5bced55c107aff2a2cabb84ebc6d980134e7ed07`. Predecessor full tests and hosted-negative evidence passed before Clippy rejected an `if status_result.is_err() { return status_result; }` form. Current head applies only `status_result?;`; no behavior or lifecycle semantics changed. Current CI is `35378675229`.

### Application-service and network lifecycle

Canonical application-service runtime/lifecycle owner #21 is exact `717edef989d8b6bb7e1f672d91d9258f7942a6b2`, Draft/open/mergeable. It owns runtime invocation identity (#20), exact post-create container lifecycle authority (#40), non-serializable private cleanup authority (#42), and successful-create malformed-stdout receipt recovery (#113). Its single-writer repair preserves those causal details in `docs/doctoring/APPLICATION_SERVICE_GAP_OWNER_REPAIR.md` and restores the repository-wide Gap file out of the leaf.

Exact #21 CI `35165254755` is classified: verify `105024795504`, production coverage `105024795320`, branch coverage `105024795584`, and hosted negative rootless/AppArmor `105024795469` are GREEN. Positive SELinux `105024795428` received no eligible `[self-hosted, linux, cwl-hostile-workload, selinux]` runner and was cancelled after 24 hours without executing steps. The workflow conclusion is therefore cancelled, not release-GREEN.

Canonical network successor #127 is now exact `71063882948daf5819839d8059c37857a6255abf`, Draft/open/mergeable, and normally contains current #21 exact `717edef...` through two-parent merge `7934aa99085264db7ee654088c3df0f116a67b66`. Before repair, fresh compare showed merge base `65f69de6...`, #127 ahead 36 and behind exactly two #21 commits; those parent-only commits were documentation/single-writer ownership repairs, not production or test semantics. The merge kept every #127 network delta while adopting `APPLICATION_SERVICE_GAP_OWNER_REPAIR.md` and #21's global-Gap restore. Docs-only `7106388...` adds `APPLICATION_SERVICE_NETWORK_OWNER_ADOPTION_TRACEABILITY.md`, recording the compare, parent set, exact #21 hosted evidence, DDD ownership boundary, and remaining gate. Current #127 CI `35384682678` is queued; historical `4306462...` CI `35302334780` and intermediate `7934aa...` CI `35384359392` do not transfer.

#127 still owns the active network sequence: exact one-object provenance → no destructive cleanup before ID admission → exact-ID non-force cleanup after admission → exact effective container attachment by admitted `NetworkID` → public correlation/private authority separation → foreign-safe explicit termination → mandatory effective isolation evidence → #128 hold/attest/release before hostile payload execution.

Application-service option/operand successor #133 is now exact `4191ac0051093c30150734f6b4e98c05f36a0ef4`, Draft/open/mergeable, normally restacked on current #127 through two-parent `b151753269beee627c30c8e9711b48c63882dddf` and then docs-only parent adoption `4191ac0...`. Its retained semantic delta remains exactly one literal `--` immediately before `request.image_reference`, with focused regression and owner-local TRACEABILITY. The restacks change no image, argv, digest, network, or cleanup semantics. Current CI `35384712828` is queued; historical #90/#91/#14 and predecessor #133 GREEN cannot be promoted.

Issue #128 remains P0: hostile application payload must not execute before effective isolation attestation. Reusable gate logic may be consumed only through stable owner integration, not mutable sibling source copying.

Issue #136/#137 remains a separate self-hosted-runner boundary. `linux-cluster-ops#326` owns s1 placement/registration and the true pre-lease enablement gate; this repository owns reusable isolation-attestation evidence. Draft #137 exact `57052b925c7f812e72e280d695aa2929a88bd85a` requires machine-verifiable RFC1918, Redis 6379, PostgreSQL 5432, and DNS denial evidence before checkout in the positive-LSM job. In-job pre-checkout evidence is defense in depth and does not prove the stronger pre-lease condition.

### Artifact-analysis owner stack

Artifact-analysis parent #18 remains exact `703b4b1a047321bb08d1d6cb1de78d01cb350696`. Do not copy unstable command/runtime source into artifact-analysis leaves; later foundation adoption must be ordinary/non-force and preserve every valid test/fixture/contract/TRACEABILITY delta.

- #53 dynamic-attestation exact `9d0da2a50e3247fd6cd2e52301d57408d404557d`: predecessor stopped at rustfmt before semantic assertions; current change is formatter-only, CI `35342943374`.
- #61 evidence-record identity exact `acb615de42d0c0013be47d47b8d2988197597100`: predecessor causally accepted foreign/duplicated job-bound evidence IDs; minimal `c024ecc...` binds canonical `<analysis_job_id>:evidence:<sequence>` and returns typed `InvalidEvidenceIdentity`; CI `35348644513`.
- #63 policy-boundary binding exact `b3204f8e1c694c18489b41a1c2a1591117e6e58f`: owner-local contract is focused current-head GREEN; later workspace failure is inherited command/runtime ancestry.
- #65 foundation-cardinality exact `3bb72c6cdbbc07823ba1c5e59f6a7b3528b29591`: formatter-only prerequisite repair, CI `35323306415`.
- #130 schema-cardinality exact `038d0ce36c04b598c246acf09a82ee2967a9aec8`: ordinary restack on current #65; production/schema remain unchanged until hardened RED executes, CI `35333036539`.
- #76 strict evidence deserialization exact `60d19a531c06f035df5291948382de7ee10484c6`: five owner-local unknown-field controls are focused GREEN; broad failure is inherited command-runtime ENTRYPOINT ancestry.
- #70 Analyzer Worker exact `991ecccabcf23380443d86873cf2777967112fbf`: Worker/Core suites are focused GREEN; the only decoded workspace failure is inherited `backend_security_info` versus expected isolation contradiction. Do not weaken worker semantics or mask with retry/sleep/mutex.
- #51/#55/#59 remain independent result/provenance/subject-binding prerequisites; #67 exact `81b4ee8d1b51dc0b47164acd290de779698d8c9b` removes only a duplicate private serializer while preserving its earlier job-identity RED.
- #102 UTF-8/dialect publication exact `ea2945fcc2d3291b18b6e7a5f6b683eff6117ba9` remains a publication RED lane. Future serialized-byte semantics require CWL nullable-field materialization followed by RFC 8785 JCS and UTF-8 octet counting. #135 Gregorian profile remains a separate dependent contract.

## DDD and ownership boundary

`sandbox_execution` is Core. `application_service` and `artifact_analysis` are Supporting bounded contexts. Podman/gVisor/containerd/Kubernetes/VM are infrastructure adapters. Generated names and serialized receipts are correlation/evidence unless a reviewed versioned contract grants stronger semantics. Destructive cleanup authority must be runtime-acquired, exact, private, and non-serializable. Requested configuration, backend-applied inspection, acquired backend identity, and live effective enforcement are distinct evidence levels.

Core foundation repositories are optional released contract owners rather than mandatory source dependencies. Consumers use immutable released contracts/ACLs; mutable branch dependencies, sibling source copies, and cross-service SQL are prohibited.

## Buyer-visible gaps

| Gap | Current authority | Required next evidence |
| --- | --- | --- |
| Branch coverage can false-underreport repeated codegen records. | #112 current `8991d4a...`; predecessor has causal RED. | Execute current exact; require complementary outcome and denominator-mismatch controls GREEN without denominator/threshold reduction. |
| Application-service network authority is not fully exact-ID and foreign-safe end to end. | #127 `7106388...`, CI `35384682678`; current #21 ancestry is adopted normally and recorded in checked-in TRACEABILITY. | Execute unchanged current exact, then progress provenance → pre-ID cleanup → post-ID cleanup → effective attachment → private authority → non-force termination → mandatory evidence. |
| Leading-dash image references need explicit Podman option termination. | #133 `4191ac0...`, CI `35384712828`, normally restacked on current #127. | Execute focused delimiter witness and prove all #127 network/lifecycle deltas remain intact. |
| Hostile application payload can precede effective isolation attestation. | #128 downstream of network lifecycle. | Hold created workload, attest exact sandbox, release only after successful effective proof, preserve cleanup/recovery evidence. |
| Future self-hosted CI may expose LAN/host services. | #136/#137 plus `linux-cluster-ops#326`. | Obtain current local RED, then reusable denial evidence plus real s1 pre-lease failed-reachability proof. |
| Dynamic analysis completeness/execution semantics are not proven end to end. | #53 formatter-clean current exact. | Execute hardened RED before any semantic GREEN. |
| Evidence IDs can contradict enclosing job/sequence. | #61 typed candidate after causal RED. | Require exact current-head typed GREEN; do not claim JSON Schema cross-instance parity. |
| EvidenceBundle schema may be weaker than Rust foundation-cardinality invariant. | #65 + #130. | Execute current RED; only then add minimal direct schema cardinality constraints. |
| Focused artifact-analysis leaves remain blocked by stale command-runtime ancestry. | #63/#76/#70 focused semantics are GREEN. | Stabilize #14/#112, then ordinary/non-force adopt into #18 and descendants; reacquire exact-head evidence. |
| Positive effective-LSM evidence is absent on release candidates. | Hosted negative evidence exists; dedicated SELinux lanes repeatedly lack runners. | Obtain exact-candidate positive effective-LSM evidence; never substitute negative fail-closed evidence. |
| No immutable consumer authority exists. | Protected `develop` unchanged; root central security non-terminal; Release inventory empty. | Resolve central gates, integrate normally, rerun protected-head CI/security/runtime evidence, then publish version/CHANGELOG, package/release, SBOM, provenance, reproducibility, rollback. |

## Current bounded action order

1. Let #112 exact `8991d4a...` execute its physical branch-union repair; do not weaken measured surface or transfer predecessor success.
2. Let #127 exact `7106388...` / CI `35384682678` execute after normal adoption of current #21 and checked-in owner-adoption TRACEABILITY; classify any remaining network failure at its intended owner boundary.
3. Let #133 exact `4191ac0...` / CI `35384712828` execute on current #127; require delimiter GREEN plus parent network/lifecycle preservation.
4. Let #53 and #61 execute current exacts before any further semantic promotion.
5. Let #65 and then #130 execute without predecessor promotion.
6. Preserve focused GREEN semantics on #63/#70/#76; stabilize #14/#112 before ordinary adoption into #18.
7. Let #51/#55/#59, current #67, and #70 children execute independently without wake commits.
8. Let #125 current exact execute the minimum Clippy repair and reacquire its own lint/rustdoc/coverage evidence.
9. Let #137 execute unchanged, then require separate real s1 pre-lease evidence from `linux-cluster-ops#326`.
10. Keep root #1 Draft until positive effective-LSM, central Dependency Review/CodeQL terminal evidence, protected integration, and immutable release evidence exist.
11. Merge/release only from one unchanged dependency-safe protected candidate satisfying every gate.
