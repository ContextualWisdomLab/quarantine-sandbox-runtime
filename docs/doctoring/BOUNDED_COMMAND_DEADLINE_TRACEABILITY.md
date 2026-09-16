# Bounded Command Deadline Traceability

Last reviewed: 2026-09-16 KST

## Scope

This note records the monotonic-deadline admission boundary for `src/infrastructure/bounded_command.rs`. It is limited to the owned subprocess runtime used by the Podman adapter. It does not change product timeout values, network isolation semantics, or public API/schema contracts.

## Problem

`BoundedCommandRunner::run` currently spawns the child process and capture workers and then evaluates `Instant::now() + self.timeout`. The runner builder accepts an arbitrary `Duration`.

Rust specifies that `Add<Duration> for Instant` may panic when the resulting point in time cannot be represented by the platform's underlying monotonic-clock representation. Rust separately exposes `Instant::checked_add`, which returns `None` for that condition. `Duration::MAX` is therefore a deterministic hostile/configuration witness for the admission boundary rather than a realistic wall-clock wait request.

A panic here is not equivalent to a typed timeout failure. It occurs after process creation and before the normal supervision/capture-cleanup path has been established, so the runtime can leave lifecycle ownership to unwinding instead of its explicit fail-closed contract.

## Current RED

PR #125 exact `52e0c16af7894bf33ac816d05b7558ce92afb14a` activates `src/infrastructure/bounded_command_deadline_tests.rs`.

`unrepresentable_command_deadline_fails_closed_without_panicking` invokes `/bin/sh -c 'exit 0'` with `Duration::MAX`, observes unwinding with `catch_unwind`, and requires both:

1. the runtime does not panic; and
2. the invocation returns a typed `Err` rather than being admitted as a valid command execution.

`catch_unwind` is test observation only. It is not an accepted production recovery mechanism and must not be introduced into the runtime.

Production deadline calculation is intentionally unchanged on this RED head. The exact-head CI must execute and fail for the intended unrepresentable-`Instant` cause before a GREEN is applied.

## Decision boundary for GREEN

The minimum repair must:

- use a non-panicking representation check such as `Instant::checked_add`;
- reject an unrepresentable deadline before unsafe lifecycle progression;
- preserve the distinction between spawn, wait, timeout, output-limit, and capture failures rather than silently clamping the requested timeout;
- keep descendant process-group termination and capture cleanup behavior unchanged for representable deadlines; and
- retain 100% owned-production line/function/region/branch coverage without exclusions or synthetic impossible-state tests.

The exact internal error mapping is intentionally deferred until the RED executes. A new error class is justified only if existing taxonomy cannot express invalid deadline admission without lying about runtime behavior.

## Rejected alternatives

- **Saturate to the largest representable instant.** Rejected because it silently changes an operator-supplied timeout into a platform-dependent effectively unbounded wait.
- **Clamp `Duration` to an arbitrary maximum.** Rejected because no product/operability authority currently defines such a maximum; inventing one here would mix policy with process runtime.
- **Wrap production execution in `catch_unwind`.** Rejected because panic recovery does not restore process ownership or prove cleanup.
- **Move directly to a fix before execution.** Rejected because the repository contract requires current-owner RED evidence before causal GREEN.

## Exact-head linkage

- bounded-command owner base: PR #72 exact `900d0273625115f7bcc602d888baadded0caeb4f`;
- process-lifecycle successor: PR #125;
- exhaustive capture-outcome repair: `cddf6e9d60a66511537495372f073f25afd31759`;
- unrepresentable-deadline test addition: `a39621fd1ed0593d3e65116d918da61eda0a312d`;
- RED activation head before this documentation commit: `52e0c16af7894bf33ac816d05b7558ce92afb14a`.

## References

The Rust Project. (2026). *Instant in std::time* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/time/struct.Instant.html

The Rust Project. (2026). *Duration in std::time* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/time/struct.Duration.html

The Rust Project. (2026). *Child in std::process* (Rust 1.98.x). Rust Documentation. https://doc.rust-lang.org/std/process/struct.Child.html
