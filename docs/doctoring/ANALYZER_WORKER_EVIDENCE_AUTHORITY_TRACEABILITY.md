# Analyzer Worker Evidence Authority Traceability

Issue #77 defines the controller-side authority boundary for evidence kinds returned by an isolated analyzer worker. It is a focused descendant of Draft #70 and does not change Core `sandbox_execution` isolation/resource/lifecycle ownership.

## Current authority

- Parent worker-port authority: Draft #70 exact `34de52819549e0362ebcd0a110a361146b3556d1`.
- Parent DDD decision: ADR-0009 remains Proposed; `artifact_analysis` owns analyzer/result semantics, `sandbox_execution` owns reusable isolation/resource/lifecycle semantics, and `infrastructure` owns concrete enforcement and observation.
- RED authority: `tests/artifact_analysis_worker_evidence_authority_red.rs`, introduced by `3b6693394a89030c286839145cbdda6aa0a83bad` and causally executed on exact `ab919126a1ada3a039fb1f8979bdb66300469d91`.
- Minimum production repair: `31dc17771b08e2535e33c8e0861c15eeb5113d4f`.
- Exact focused GREEN authority: `fea4594468ad273599d97040d71e07b2d4e67faf`, native CI `34314242415`, verify `102347001047`.
- Repository-CI authority: root Draft #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`; this descendant adopts its checkout credential policy rather than forking it.
- Rustdoc repair: `c6b9a93762d62bad3f89286285932bf594d6f3c5`, documentation-only over the same production semantics.

At the executed RED head, the worker finding contract carried the repository-wide `EvidenceKind` and `AnalyzerWorkerReceipt::validate_against` validated bounded text/attributes plus request and Core isolation consistency, but did not constrain which evidence authorities an untrusted analyzer could claim. An otherwise-valid worker could therefore label one normalized finding as `ArtifactIdentity` or `PolicyBoundary` even though those facts are assembled from controller-admitted immutable bytes and runtime-owned boundary state.

## Decision

Treat evidence kind as an authority-bearing field, not merely an enum-shaped label. The controller-owned worker-result ACL rejects analyzer claims for evidence categories whose truth source belongs to the controller or runtime.

The first minimum boundary is deliberately narrow:

- `ArtifactIdentity` is controller-owned because immutable subject identity comes from admitted artifact bytes.
- `PolicyBoundary` is runtime/controller-owned because isolation and policy-boundary truth comes from runtime-observed state rather than analyzer self-report.
- `FileFormat` remains admissible because the product already has analyzer-produced non-executing format classification.
- `StaticCapability` remains analyzer-owned.
- `RuntimeBehavior` and `NetworkAttempt` are not made generally admissible by this issue; issue #52/#53 owns dynamic-execution truthfulness and completeness.
- `ToolFailure` remains attributable failure evidence and is not redefined here.

Forbidden kinds fail closed. They are not rewritten into another kind, dropped silently, or authorized merely because the enclosing worker receipt matches its request.

## Executed RED and minimum GREEN

Native CI `34306038425`, verify `102322814270`, executed exact `ab919126a1ada3a039fb1f8979bdb66300469d91`. Exact checkout, dependency lock, repository validation, coverage-parser tests, rustfmt, Core worker-isolation suites, and `worker_owned_static_evidence_remains_admissible` all passed. `worker_cannot_claim_controller_or_runtime_owned_foundation_evidence` then failed for the intended cause: an `ArtifactIdentity` finding returned `Ok(())` where the contract required `AnalyzerWorkerContractError::InvalidOutcome { field_name: "evidence_kind" }`. The adjacent static-evidence case remaining GREEN rules out a repair that indiscriminately rejects worker findings.

Minimum production repair `31dc17771b08e2535e33c8e0861c15eeb5113d4f` adds one fail-closed guard in `AnalyzerWorkerFinding::validate`: `ArtifactIdentity | PolicyBoundary` returns `InvalidOutcome { field_name: "evidence_kind" }` before worker output is admitted. It leaves `FileFormat`, `StaticCapability`, `RuntimeBehavior`, `NetworkAttempt`, and `ToolFailure` behavior otherwise unchanged and does not move the taxonomy into Core or add backend-specific state.

Exact `fea4594468ad273599d97040d71e07b2d4e67faf` then reacquired the focused GREEN in native CI `34314242415`. Verify `102347001047` passed exact checkout, dependency lock, repository policy, coverage-parser tests, rustfmt, both producer-authority regressions, and all five `ci_runner_contract` tests including checkout credential disposal. The same verify job later failed in inherited command-runtime test `cleanup_failure_is_not_hidden_behind_container_start_failure` with `BackendInvocationFailed { operation: "backend_security_info" }` instead of the downstream cleanup assertion. That rotating process-boundary failure is owned by issue #71 / Draft #72; it is not evidence that the artifact-analysis ACL failed. No broad or predecessor GREEN is transferred.

## Repository CI ownership repair

The causal RED run on `ab919126...` also exposed stale descendant CI authority: `actions/checkout` executed with `persist-credentials: true`. The canonical root already sets `persist-credentials: false` on every checkout and carries `every_checkout_discards_persisted_credentials` so build/test steps do not retain ambient Git write credentials.

This is independent of analyzer semantics and is repaired through the canonical `.github` owner contract rather than a local exception. Test-only `d03df966f171d5ce445ad0240ff91bf32727ec7b` adopts the root `tests/ci_runner_contract.rs` blob exactly. Workflow repair `13969067ee0ba9d8dd90d9706018b638025eb00b` adopts the root `.github/workflows/ci.yml` blob exactly, including `persist-credentials: false` on all five checkout steps. Those commits do not change production Rust, runner labels, toolchains, test commands, or the evidence-authority decision. Exact `fea459...` executed the adopted invariant successfully.

Keeping persisted credentials was rejected: this repository's CI permissions are read-only and later Rust/tests need no Git write authority. Maintaining a descendant-specific weaker workflow was also rejected because `.github` is the canonical CI/security owner and source-copy divergence would recreate the same gap.

## Review and rustdoc repair

CodeRabbit's fresh review of exact `fea4594468ad273599d97040d71e07b2d4e67faf` generated no actionable code comments and assessed the merge risk as minimal, but its pre-merge quality check reported only 33.33% docstring coverage across 15 touched functions. The repository execution contract is stricter: owned production rustdoc must be complete, including private validation helpers whose security role is not obvious from their signatures.

Documentation-only `c6b9a93762d62bad3f89286285932bf594d6f3c5` adds rustdoc to the previously undocumented production validators and predicates in `src/artifact_analysis/analyzer_worker.rs`: stored analyzer-identity revalidation, controller-request revalidation, worker evidence-authority validation, semantic-outcome validation, bounded identity text, bounded/control-free text, and exact lower-case SHA-256 admission. It changes no executable expression, type, public API, policy decision, test, workflow, or backend behavior. A fresh exact-head review/CI result is still required after this documentation change; the predecessor's broad failing state is not converted into merge authority merely because the focused slice had already gone GREEN.

## Security rationale

NIST SP 800-53 Rev. 5.1 control SI-10 requires systems to validate information inputs against defined syntax, semantics, and acceptable values. For this boundary, the acceptable value set is context-sensitive: an enum value can be syntactically valid while semantically unauthorized for an untrusted analyzer producer. The controller therefore validates both the shape and the authority of worker-supplied evidence metadata.

Saltzer and Schroeder's complete-mediation and fail-safe-default principles support the same boundary: authority is checked at the point where an untrusted result crosses into trusted controller state, and absence of an explicit producer authority must not become implicit permission.

## Alternatives rejected

- Accept every `EvidenceKind` because it is a public enum: rejected because type membership does not establish producer authority.
- Infer authority from `producer_id`: rejected because producer display identity is not the trust root for foundation/runtime facts.
- Rewrite forbidden kinds into `StaticCapability` or `ToolFailure`: rejected because silent coercion destroys provenance and can turn a security contradiction into apparently valid evidence.
- Move the enum or authority table into `sandbox_execution`: rejected because evidence semantics belong to Supporting `artifact_analysis`; Core owns isolation/resource/lifecycle truth, not analyzer evidence taxonomy.
- Solve `RuntimeBehavior` / `NetworkAttempt` in this slice: rejected because #52/#53 already owns the dynamic-execution/completeness contract and combining it would blur the causal RED.
- Keep persisted checkout credentials or a descendant-specific workflow exception: rejected because neither analyzer execution nor repository validation requires ambient Git write credentials, and `.github` is the canonical CI/security owner.
- Silence the docstring warning by lowering the external review threshold: rejected because the repository's owned-production rustdoc requirement is stricter and the undocumented functions encode trust-boundary decisions.

## References

Joint Task Force. (2020/2025). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5.1), SI-10 Information Input Validation. National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Saltzer, J. H., & Schroeder, M. D. (1975). The protection of information in computer systems. *Proceedings of the IEEE, 63*(9), 1278–1308. https://doi.org/10.1109/PROC.1975.9939

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Completion gate

The final unchanged head must execute the focused producer-authority positive/negative cases and the checkout-credential regression, then pass repository validation, rustfmt, full workspace tests, Clippy with warnings denied, rustdoc with warnings denied, exact complete owned-production statement/function/region/branch coverage, qualifying review/security/thread gates, dependency-safe adoption of the stabilized command/runtime ancestry through #18/#70, applicable positive effective-isolation evidence, protected integration, SBOM/provenance/reproducibility/rollback, and immutable publication. Predecessor results never transfer to a moved head.