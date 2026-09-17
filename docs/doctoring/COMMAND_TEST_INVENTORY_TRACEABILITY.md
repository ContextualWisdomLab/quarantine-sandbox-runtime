# Native command test inventory

Status: development-stack repair on canonical PR #14, not protected integration or release.

## Executed parent return

Exact namespace child `6fd981c401d50ad96686152efaac7063f6317808`, native CI `34436853392`, verify job `102743620215`, executed the following slices successfully: library 36/36, CLI 18/18, primary command 15/15, cleanup 5/5, missing UTS/cgroup evidence 2/2, exact entrypoint 1/1, and trusted-gate ACK/stdin 2/2. The next target failed at `podman_command_execution_hold_gate_binding_red.rs:109`: create still selected consumer argv as OCI entrypoint instead of `/qsr-runtime-gate`. The job exited 101. Later targets were not executed by that invocation; absence of output is not success.

On September 10, 2026, the canonical unprotected `feat/podman-command-execution-backend` ref was advanced normally from `645e05825d9991f3054a69efb9dd4231a55e95ce` to the exact child head. GitHub records PR #108 as merged at `2026-09-10T05:00:30Z`. This preserves every namespace test, production predicate, fixture correction and historical parent. Protected `develop` and production consumers were not changed. Issue #25 remains open and requires the complete hold/attest/release composition.

## Evidence gap and minimum repair

Cargo's default behavior stops after the first failing test executable. Native verify used `cargo test --locked --workspace --all-targets`, so the inherited hold-gate RED concealed later command lifecycle and gate-artifact failures. The existing runbook already offered `--no-fail-fast` locally but native CI did not execute it.

Only the existing native Test command gains `--no-fail-fast`. It continues to select the whole workspace and all targets with the locked dependency graph. No test-name filter, ignored-test shortcut, `continue-on-error`, shell failure suppression, new job, extra workflow, runner change or permission change is introduced. The final failing status remains authoritative. Existing real-Podman lanes and coverage thresholds are unchanged; the ordinary target sweep does not execute ignored real-container tests and is not a substitute for those lanes.

`scripts/test_ci_test_inventory.py` checks the actual native workflow, protects its complete target command and failure status, and requires its own execution before Rust tests. The existing coverage-parser unittest step also runs this contract. These are repository configuration tests, not production Python logic or Rust runtime tests.

## RED / GREEN and counterexamples

Against the authenticated original workflow blob `1f5bdf5cd3b73ccdbee48d2778ebcf3b4ffad467`, the new three-test contract suite produced two assertion failures: missing `--no-fail-fast` and absent CI self-wiring. After the three-line workflow repair, all three tests passed. Five independent mutations were then rejected: removing the flag, appending `|| true`, enabling job-level `continue-on-error`, removing self-wiring, and filtering the held-gate tests. Restoring the candidate returned the suite to 3/3 passing. Python byte compilation also passed.

No local Cargo, rustc or Podman was available. These results establish the workflow contract only. The new exact-head native run must still demonstrate later targets executing after the first real failure and a nonzero final status. Any failure it reveals is repaired at its owning runtime boundary; assertions and coverage denominators are not relaxed.

## Reproduction and parent acceptance

Run `python3 -m unittest scripts/test_ci_test_inventory.py -v` from the repository root. The source-preserving full Rust inventory command is maintained in [OPERABILITY](../OPERABILITY.md#command-namespace-fixture-failures).

Namespace adoption is complete only within the development stack. Parent #14 still requires trusted-gate artifact admission, live effective isolation before payload execution, bounded one-time release, trusted ACK, detached consumer stdin, exact argv execution, exact-ID cleanup, full owned coverage, current review/security and dedicated positive-LSM evidence before protected integration or release. Child #109's loader-boundary work remains separate until its full delta is verified and integrated.

## Primary references

ContextualWisdomLab. (2026, September 10). *CI at 6fd981c401d50ad96686152efaac7063f6317808* [Workflow job 102743620215]. GitHub. https://github.com/ContextualWisdomLab/quarantine-sandbox-runtime/actions/runs/34436853392/job/102743620215

Rust Project Developers. (n.d.). *cargo test*. The Cargo Book. Retrieved September 10, 2026, from https://doc.rust-lang.org/cargo/commands/cargo-test.html
