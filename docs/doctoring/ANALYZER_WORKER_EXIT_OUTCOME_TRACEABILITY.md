# Analyzer Worker Exit / Outcome Traceability

Status: Proposed / RED-only

Issue: #79

Parent authority: Draft #70 exact `86e86c9c5a13f287e341a6485e6acc5f61cee811`

Test-bearing authority: `a23fa5a1ab2bf2d5b055a21d6f614741cbb55990`

## Problem

`SandboxWorkerTerminationState::Exited { exit_code }` is runtime-owned Core evidence, while `AnalyzerWorkerOutcome::Completed` is Supporting-context semantic evidence. Current `AnalyzerWorkerReceipt::validate_against` requires only a terminal state for the exact worker. It does not reject `Completed` when the runtime-observed process exit code is nonzero.

That gap permits partial or buffered analyzer output from an abnormal/non-successful worker execution to cross the controller ACL as a trustworthy completed result. It is distinct from downstream issue #56/#57, which constrains `ToolFailure` evidence against bundle-level `RuntimeDisposition::Completed`; #79 binds the worker process disposition before downstream evidence assembly.

## Decision

The controller-side analyzer-worker ACL must admit `AnalyzerWorkerOutcome::Completed` only when runtime-owned termination evidence is `SandboxWorkerTerminationState::Exited { exit_code: 0 }`. A nonzero exit with `Completed` must fail closed as `AnalyzerWorkerContractError::InvalidOutcome { field_name: "worker_exit_code" }`.

`AnalyzerWorkerOutcome::Failed` remains a semantic failure channel and is not required by this slice to have a nonzero process exit. Core continues to own termination observation; `artifact_analysis` owns the cross-field mapping from that observation to analyzer outcome semantics. Infrastructure remains responsible for observing backend-specific process disposition without promoting partial output to completion authority.

## RED

`tests/artifact_analysis_worker_exit_outcome_red.rs` constructs two otherwise-identical receipts with every current required isolation control verified, exact worker identity, matching policy SHA-256 and budget, terminal cleanup, and `Completed { findings: vec![] }`.

- `exit_code: 137` must be rejected as `InvalidOutcome { field_name: "worker_exit_code" }`.
- `exit_code: 0` remains admissible.

Current production has no such cross-field predicate, so the first case is intentionally RED until exact-head CI executes it for this cause.

## Alternatives rejected

Treating every terminal state as completion authority is rejected because terminality establishes lifecycle closure, not success. Converting every nonzero exit to `AnalyzerWorkerOutcome::Failed` inside Core is rejected because Core must not own analyzer semantics. Requiring every semantic `Failed` outcome to have a nonzero exit is also rejected because a worker can complete its process normally while returning a typed analyzer-level failure.

## Minimum GREEN after causal RED

Add the smallest controller-owned check in `AnalyzerWorkerReceipt::validate_against` (or an equivalent private `artifact_analysis` predicate) that rejects `Completed` unless the exact worker termination state is `Exited { exit_code: 0 }`. Do not change Core taxonomy, backend selection, aggregate result-channel limits, evidence-kind producer authority, or dynamic-execution semantics.

## Evidence and references

Rust Project. (2026). *std::process::ExitStatus* (Rust 1.98.x). The standard library defines successful termination as zero exit status and does not treat signal termination as success. https://doc.rust-lang.org/std/process/struct.ExitStatus.html

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Joint Task Force. (2022). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53Ar5

NIST SI-10/assessment procedures support fail-closed validation of security-relevant inputs and cross-field constraints. Here the runtime-owned exit status and Supporting-context completion claim are jointly security-relevant evidence and must not be accepted when contradictory.
