# Bounded Command Completion Traceability

Last reviewed: 2026-09-13

## Decision

`BoundedCommandRunner::run_to_completion` reports terminal process state as the closed sum type `BoundedCompletion::{Exited(ExitStatus), TimedOut, OutputLimit}`. It does not represent terminal cause as independent `Option<ExitStatus>` and timeout booleans.

The previous product type admitted contradictory states that the supervisor could not produce. In particular, `wait_for_command` first handled timeout and retained-output overflow, then still contained a later `status=None => BackendInvocationFailed` arm. Coverage review established that manufacturing a test input for that arm would violate the supervisor contract rather than exercise a hostile workload.

## Causal repair

Source `49337a2ffec3edd7492295c7dd8e5144b884fe18` introduced `BoundedCompletion` and exhaustive consumers. Wall-clock timeout remains distinct from output-budget termination; exited non-success remains a backend-command failure; command-wait truncation retains precedence for exited processes; `Wait` and `Capture` remain hard supervisor/process-boundary failures.

One-shot run `34727202814` validated rustfmt, the full locked workspace/all-target test suite, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check` before publishing that source. Purpose-complete source-fix machinery was subsequently removed.

Review found stale rustdoc describing the removed `None`/`timed_out` representation. After repairing a malformed temporary workflow definition, one-shot run `34727502889` validated and published docs-only source `856d91c9b31c7bcdc9f5aae2dd3d0aa6032e4c16`; the workflow was then removed. `docs/product-technical-gap-baseline.md` now carries the corresponding current-authority supersession.

## Exact immutable evidence

Executable-source authority `343c77a66746b382cfcc1633691d4fa70af3a95a` ran native CI `34727579849`. `verify` and hosted negative rootless/AppArmor completed GREEN. Production and branch coverage generated immutable artifacts and failed only repository-wide 100% admission:

- production artifact `10308935247`, `sha256:5015b9b5a9f8e3f55f93116581221ea91e2e538b0cbb858de37760054844de01`;
- branch artifact `10308241142`, `sha256:76742b437c9c53b23f7ddde18f5567c73f279fb541917453673487e3b3b63af3`;
- 5209/5251 lines, 502/502 functions, 6982/7109 regions, and 717/728 branches.

The typed repair therefore preserves 100% production-function coverage while removing two branch outcomes from the earlier contradictory state product. It does not establish repository-wide line/region/branch completion.

## Late overflow canonicalization

Exact branch evidence on `343c77a...` left `bounded_command.rs` at 19/20 branches. The missing decision was not an impossible input: a child can exit before the drain worker records retained-output overflow. The repair preserves that race-safety requirement without a scheduler-sensitive test by moving output-limit classification to the point after both drain workers have joined. When the supervisor observes the overflow first it now kills/reaps and returns the reaped status; final classification then uses the retained overflow flag. When child exit wins the race, the same post-drain classification produces the same `OutputLimit`. Timeout and supervisor `Wait` failures keep their existing precedence because only a successful supervised status is reclassified.

The existing concrete oversized-output test therefore exercises the same production decision deterministically regardless of whether child exit or the supervisor overflow observation wins the scheduling race. No sleep tuning, process-global mutation, coverage exclusion, or synthetic impossible state is introduced.

Application-service readiness/lifecycle outcomes and the receipt/exact-ID, independent capability-column, and duplicate network-inspection findings remain canonical #21/#113 work.

## Capture-precedence repair

Review of exact source `10c97e5bbdd4f2e1d8e7494ec853e640bc9b788f` found a semantic regression in the first late-overflow canonicalization. That source normalized a successful supervised status into `OutputLimit` before administrative output finalization. A drain worker can first set the overflow flag and then fail a later pipe read; in that state the pre-normalized status masked the harder `Capture` failure that the previous finalizer ordering preserved. The causal RED combines the real overflow supervisor outcome with a capture failure and reaches `Err(OutputLimit)` instead of the required `Err(Capture)`.

The repair keeps overflow detection deterministic after both drain workers join but defers overflow interpretation to each consumer. Administrative `run` preserves supervisor-error precedence, then pipe-capture precedence, then classifies late overflow. `run_to_completion` first preserves pipe-capture failure, then classifies timeout/wait before late overflow, so supervisor malfunction is never rewritten as a workload terminal fact. No sleep tuning, process-global mutation, coverage exclusion, or output-budget weakening is introduced.

One-shot run `34728500402` executed the causal RED first: the focused regression failed on the pre-repair state with `OutputLimit` where `Capture` was required. The same run then applied the minimal ordering repair and completed the focused GREEN, full locked workspace/all-target test suite, Clippy with warnings denied, rustdoc with warnings denied, rustfmt, and `git diff --check` before publishing executable source `1eb6965a7d8bcb7601e23a095badaf8b739b0968`. The temporary source-fix workflow removed itself in that commit. GitHub then classified native CI `34728569622` on the bot-authored source commit as `action_required` with no transferable exact-head GREEN; this ordinary documentation descendant exists to rematerialize native CI without changing executable semantics.

## Release consequences

No predecessor GREEN transfers after a head move. Merge/release still requires exact-current-head native verification, complete owned-production coverage, dedicated positive effective-LSM evidence, qualifying independent review/security, #35/#43 real-runtime acceptance, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback evidence.

## Primary references

The Rust Project. (2026). *std::process::ExitStatus*. Rust standard library documentation. https://doc.rust-lang.org/std/process/struct.ExitStatus.html

The Rust Project. (2026). *std::process::Output*. Rust standard library documentation. https://doc.rust-lang.org/std/process/struct.Output.html
