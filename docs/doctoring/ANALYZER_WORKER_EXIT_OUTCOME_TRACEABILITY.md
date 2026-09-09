# Analyzer Worker Exit / Outcome Traceability

Status: Proposed / candidate GREEN

Issue: #79

Parent authority: Draft #70 exact `34de52819549e0362ebcd0a110a361146b3556d1`

RED test authority: `a23fa5a1ab2bf2d5b055a21d6f614741cbb55990`

Minimum production candidate: `f570f5fd4329f9a193280a6b07233bcf3ea73b83`

Semantic-failure edge regression: `de4f89ca698a76f83ac0179952fc43a3c97a314f`

Repository-CI authority: root Draft #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`

## Problem

`SandboxWorkerTerminationState::Exited { exit_code }` is runtime-owned Core evidence, while `AnalyzerWorkerOutcome::Completed` is Supporting-context semantic evidence. At the executed RED head, `AnalyzerWorkerReceipt::validate_against` required only a terminal state for the exact worker and did not reject `Completed` when the runtime-observed process exit code was nonzero.

That gap permitted partial or buffered analyzer output from an abnormal/non-successful worker execution to cross the controller ACL as a trustworthy completed result. It is distinct from downstream issue #56/#57, which constrains `ToolFailure` evidence against bundle-level `RuntimeDisposition::Completed`; #79 binds worker process disposition before downstream evidence assembly.

## Decision

The controller-side analyzer-worker ACL admits `AnalyzerWorkerOutcome::Completed` only when runtime-owned termination evidence is `SandboxWorkerTerminationState::Exited { exit_code: 0 }`. A nonzero exit with `Completed` fails closed as `AnalyzerWorkerContractError::InvalidOutcome { field_name: "worker_exit_code" }`.

`AnalyzerWorkerOutcome::Failed` remains a semantic failure channel and is not required by this slice to have a nonzero process exit. Core continues to own termination observation; `artifact_analysis` owns the cross-field mapping from that observation to analyzer outcome semantics. Infrastructure remains responsible for observing backend-specific process disposition without promoting partial output to completion authority.

## Executed RED

Native CI `34306130118`, verify `102323088196`, executed exact `1cc33fa924678b7c0fca6fcc830eda256c90946d`. Exact checkout, dependency lock, repository policy, coverage-parser tests, rustfmt, library tests, application-service tests, host-capability regression, and Core worker-boundary tests passed before the focused worker-exit tests ran.

`completed_worker_outcome_accepts_zero_runtime_exit` passed. `completed_worker_outcome_rejects_nonzero_runtime_exit` then failed for the intended cause: an otherwise-valid receipt with runtime-observed `Exited { exit_code: 137 }` and `Completed { findings: vec![] }` was admitted instead of returning `InvalidOutcome { field_name: "worker_exit_code" }`. Hosted negative rootless/AppArmor `102323088158` also passed on that exact head; coverage and branch-coverage stopped on the same semantic RED, and positive-LSM remained queued.

## Minimum causal GREEN candidate

Production candidate `f570f5fd4329f9a193280a6b07233bcf3ea73b83` adds one Supporting-context cross-field check in `AnalyzerWorkerReceipt::validate_against`: after Core isolation/lifecycle validation succeeds, `Completed` is rejected unless the exact runtime termination state is `Exited { exit_code: 0 }`. The change does not alter Core termination taxonomy, backend selection, aggregate result-channel limits, evidence-kind producer authority, or dynamic-execution semantics.

Edge-regression commit `de4f89ca698a76f83ac0179952fc43a3c97a314f` additionally proves the rejected alternative was not introduced: `AnalyzerWorkerOutcome::Failed` remains admissible when an otherwise-valid worker exits zero because semantic analyzer failure and process failure are distinct dimensions.

Exact `0d20d05e51900ba2b04816ff9358370974b9fc48` moved the source/test candidate to code-current doctoring. Native CI `34309057623` materialized but was still queued when the branch moved for repository-CI hardening; no GREEN transfers from it.

## Repository CI ownership repair

Fresh inspection of `0d20d05...` found the descendant still omitted canonical checkout credential disposal: every `actions/checkout` step defaulted to `persist-credentials: true`, and its `tests/ci_runner_contract.rs` lacked the root regression. Root #1 already owns the stricter repository contract, so this is a stale `.github` ancestry finding rather than an artifact-analysis exception.

Test-only `7c93241fa5e5956e36bb6d7bc0702e1c676394fb` adopts the root `tests/ci_runner_contract.rs` blob exactly, including `every_checkout_discards_persisted_credentials`. Workflow repair `366d278116790974014c07c564234e0e373cee5e` adopts the root `.github/workflows/ci.yml` blob exactly and sets `persist-credentials: false` on all five checkout steps. Neither commit changes production Rust, runner labels, toolchains, tests, isolation semantics, or the exit/outcome decision.

Keeping persisted credentials is rejected because repository CI has read-only content permission and later test/build steps do not require ambient Git write authority. A descendant-specific weaker workflow is also rejected because `.github` is the canonical CI/security owner.

No GREEN is claimed until one unchanged exact descendant executes the exit/outcome cases and checkout-credential regression, then full fmt/test/Clippy/rustdoc, complete owned-production/branch coverage, applicable review/security/confinement gates, and parent/protected integration.

## Alternatives rejected

Treating every terminal state as completion authority is rejected because terminality establishes lifecycle closure, not success. Converting every nonzero exit to `AnalyzerWorkerOutcome::Failed` inside Core is rejected because Core must not own analyzer semantics. Requiring every semantic `Failed` outcome to have a nonzero exit is also rejected because a worker can complete its process normally while returning a typed analyzer-level failure. Keeping stale persisted checkout credentials is rejected because CI does not need that write-capable ambient authority.

## Evidence and references

Rust Project. (2026). *std::process::ExitStatus* (Rust standard library). Successful process status is represented separately from unsuccessful termination; this Supporting-context mapping uses the runtime-observed numeric exit code already present in the repository's Core evidence contract. https://doc.rust-lang.org/std/process/struct.ExitStatus.html

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Joint Task Force. (2022). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53Ar5

NIST SI-10/assessment procedures support fail-closed validation of security-relevant inputs and cross-field constraints. Here the runtime-owned exit status and Supporting-context completion claim are jointly security-relevant evidence and are not accepted when contradictory.

## Completion gate

The final unchanged head must execute the nonzero-completed rejection, zero-completed acceptance, zero-exit semantic-failure acceptance and checkout-credential regression, then repository validation, rustfmt, full workspace tests, Clippy/rustdoc, exact complete owned-production statement/function/region/branch coverage, qualifying review/security/thread gates, dependency-safe stabilized runtime ancestry through #18/#70, applicable positive effective isolation, protected integration, SBOM/provenance/reproducibility/rollback and immutable publication. `docs/product-technical-gap-baseline.md` must be code-current before completion; predecessor/queued/cancelled runs do not transfer.
