# Bounded Command Deadline Traceability

Last reviewed: 2026-09-16 KST

## Scope

This note records the monotonic-deadline admission boundary for `src/infrastructure/bounded_command.rs` and the exact coverage edge exposed while verifying that repair. It is limited to the owned subprocess runtime used by the Podman adapter. It does not change product timeout values, network isolation semantics, or public API/schema contracts.

## Executed causal evidence

PR #125 exact `f58dfc6657745b3431502f617c7febad6737efcd`, CI `35047105870`, executed the active `unrepresentable_command_deadline_fails_closed_without_panicking` witness on hosted Ubuntu 24.04 / Rust 1.97.1.

The coverage job reached the witness with 16 sibling bounded-command tests GREEN and failed at the intended cause: `BoundedCommandRunner::run` panicked at `src/infrastructure/bounded_command.rs:75` with `overflow when adding duration to instant`. The test-side `catch_unwind` then failed its no-panic assertion. This is current-owner causal RED evidence rather than a source-only inference.

The same exact head independently exposed a rustfmt-only failure in `src/infrastructure/bounded_command_deadline_tests.rs`. Commit `ffe224b5cd3e948228c82c24d4b0f93604fc22b0` applies only that formatter delta.

## Causal deadline repair

Production commit `1709c41c0c88a378bd0b21889a7b9c2987fd93c5` replaces the panicking `Instant::now() + self.timeout` path with `Instant::checked_add` and performs deadline admission before `spawn_piped_child`.

An unrepresentable wall-clock budget therefore returns the existing typed `BoundedCommandError::Timeout` before a child process or capture worker is created. The existing timeout class was retained rather than introducing a second public/internal deadline taxonomy: both conditions mean the requested bounded-command wall-clock budget cannot be completed under the runtime contract, while spawn, wait, output-limit, and capture failures remain distinct. The enum rustdoc states that `Timeout` covers an unrepresentable or exceeded wall-clock budget.

This repair intentionally does not saturate or clamp the requested `Duration`, does not use production `catch_unwind`, and does not rely on cleanup after a panic. Representable invocations continue through the existing process-group supervision and shared capture deadline; the deadline is fixed before process creation so spawn and capture setup are included in the same wall-clock budget.

## Exact repair execution and coverage RCA

PR #125 exact `f5b26660761a283d4dc9bf5b2c14947b51f4b946`, CI `35072937902`, coverage job `104718486946`, then executed the repaired runtime on hosted Ubuntu 24.04 / Rust 1.97.1. All 17 bounded-command tests passed, including `unrepresentable_command_deadline_fails_closed_without_panicking`, successful execution, timeout, output-limit, and descendant-held-pipe controls. This validates the causal `checked_add` repair on that exact head; it does not by itself make the whole PR GREEN.

The same exact job failed only at the repository 100% coverage gate. The normalized evidence was `2219/2222` lines and `3004/3005` regions, with `205/205` functions. The only incomplete production file was `src/infrastructure/bounded_command.rs` (`473/476` lines, `754/759` LLVM regions), and the first uncovered segment began at `232:69`.

That segment is the cleanup-failure arm inside `CaptureWaitOutcome::OutputLimit`:

```rust
child.terminate().map_err(|_| BoundedCommandError::Wait)?;
```

Existing tests already exercised successful output-limit termination and deadline-path termination failure, but not the combined state in which the supervised child has exited, a pipe remains open through a detached descendant, retained output overflows, and terminating the original process group fails because that group no longer exists. The uncovered edge therefore represented a missing lifecycle test, not dead production code and not a reason to weaken the denominator.

The exact coverage artifact was `coverage-json-f5b26660761a283d4dc9bf5b2c14947b51f4b946`, artifact id `10444683520`, SHA-256 `32c0546aae4a23c2d297e8465321a3f7f59016238eeadc7e31790c2b996d5142`.

## Coverage edge repair

Test-only commit `baa4c481116decbd708a84bd3b5a1b34d50b978e` adds `detached_descendant_output_limit_preserves_cleanup_failure` to `src/infrastructure/bounded_command_deadline_tests.rs` on Linux. The fixture starts a detached `setsid` descendant which waits until the supervised shell exits, writes more than the retained-output budget, and keeps the inherited pipe open. The bounded runner must then surface `BoundedCommandError::Wait` when output-limit handling tries to terminate the already-gone original process group.

This is a direct hostile lifecycle witness for the previously uncovered cleanup edge. It adds no coverage exclusion, ignores no test, and changes no production semantics. Production source remains identical to `1709c41c0c88a378bd0b21889a7b9c2987fd93c5`.

The documentation successor after that test-only repair is the current PR head produced by this note. Exact-head GREEN must be established again on that unchanged successor; predecessor execution does not transfer.

## Remaining verification

The current successor must still execute:

- the `Duration::MAX` witness without panic and with typed failure;
- the detached-descendant output-limit cleanup-failure witness and all existing successful/timeout/output-limit/descendant-held-pipe process tests;
- `cargo fmt --check`, locked full workspace/all-target tests, Clippy `-D warnings`, public/private rustdoc `-D warnings`;
- exact owned-production statement/function/region/branch and edge coverage at 100%; and
- applicable hosted/runtime security gates.

Positive SELinux evidence remains a separate release gate and predecessor status does not transfer to a moved head.

## Decision record

**Problem.** `Instant + Duration` could panic after subprocess creation when the requested monotonic deadline was not representable. After the causal repair executed successfully, exact coverage exposed one previously untested output-limit cleanup-failure edge.

**Constraints.** Fail closed before lifecycle side effects, preserve bounded-command failure distinctions, keep one deadline across child supervision and pipe capture, do not silently mutate policy, and retain exact 100% production coverage without exclusions.

**Rejected alternatives.** Saturating to the largest representable instant or clamping to an arbitrary duration changes operator intent. Production `catch_unwind` does not restore process ownership. Computing a checked deadline only after spawning still permits lifecycle side effects before admission. Excluding the uncovered segment or weakening coverage would hide a real hostile-process lifecycle state.

**Selected direction.** Admit one checked deadline before spawn and reuse it through supervision/capture completion. Preserve the existing `Timeout` class as the wall-clock-budget failure category. Cover the output-limit cleanup-failure arm with a concrete detached-descendant process fixture rather than modifying production solely for coverage.

**Effect.** Unrepresentable durations fail as typed runtime errors without creating a child; representable deadlines cover the complete bounded invocation; the remaining cleanup edge has an explicit hostile lifecycle witness awaiting exact-head execution.

## Exact-head linkage

- bounded-command owner base: PR #72 exact `900d0273625115f7bcc602d888baadded0caeb4f`;
- process-lifecycle successor: PR #125;
- exhaustive capture-outcome repair: `cddf6e9d60a66511537495372f073f25afd31759`;
- unrepresentable-deadline test addition: `a39621fd1ed0593d3e65116d918da61eda0a312d`;
- RED activation: `52e0c16af7894bf33ac816d05b7558ce92afb14a`;
- executed RED / pre-repair evidence: `f58dfc6657745b3431502f617c7febad6737efcd`, CI `35047105870`;
- exact rustfmt repair: `ffe224b5cd3e948228c82c24d4b0f93604fc22b0`;
- minimum production repair: `1709c41c0c88a378bd0b21889a7b9c2987fd93c5`;
- executed repair / coverage-RCA head: `f5b26660761a283d4dc9bf5b2c14947b51f4b946`, CI `35072937902`, coverage job `104718486946`;
- output-limit cleanup-failure coverage witness: `baa4c481116decbd708a84bd3b5a1b34d50b978e`.

## References

The Rust Project. (2026). *Instant in std::time* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/time/struct.Instant.html

The Rust Project. (2026). *Duration in std::time* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/time/struct.Duration.html

The Rust Project. (2026). *Child in std::process* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/process/struct.Child.html

bytecodealliance. (2026). *kill_process_group in rustix::process*. docs.rs. https://docs.rs/rustix/latest/rustix/process/fn.kill_process_group.html

The Open Group. (2024). *kill — send a signal to a process or a group of processes*. POSIX.1-2024.
