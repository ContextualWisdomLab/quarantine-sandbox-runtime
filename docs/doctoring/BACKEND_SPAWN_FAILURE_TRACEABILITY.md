# Backend Spawn Failure Traceability

## Decision status

Proposed on 2026-09-07. Issue #71 and Draft PR #72 own this focused observability repair above the current runtime foundation. This document records stable causal evidence and design constraints only; transient queued/running job identifiers belong in PR/Issue/central owner-path metadata and are not versioned evidence.

## Problem and causal evidence

`src/infrastructure/bounded_command.rs` currently maps every `std::process::Command::spawn()` failure to one `BoundedCommandError::Spawn` value and also uses that same value when expected stdout/stderr pipes are absent. `src/infrastructure/podman.rs` then maps `Spawn`, `Wait`, and `Capture` to `ApplicationServiceError::BackendInvocationFailed { operation }`. The resulting application-service error identifies the failed operation but discards the causal OS failure class.

Old exact native-CI specimens on #9 and #10 observed `BackendInvocationFailed { operation: "backend_security_info" }` before the downstream semantic assertion. Those observations did not establish whether the cause was a missing executable, permission denial, resource pressure, or another invocation failure, so they do not authorize a retry policy.

The first #72 RED requested a stable `BackendInvocationFailureKind::NotFound` and `BackendSpawnFailed` result for a missing executable. Review found that a fixed process-ID-derived pathname directly under shared `/tmp` was only conventionally absent. The test now creates a directory it exclusively creates for that attempt and targets a child executable path that is absent inside that directory. Production behavior remains unchanged until this deterministic RED executes for the intended missing classification API/variant cause.

## DDD boundary

- `infrastructure::bounded_command` owns translation from OS/process-spawn failures to a backend-neutral internal spawn classification.
- `application_service` may expose only the small, stable failure vocabulary required for operability and controller decisions.
- Raw errno values, host paths, OS error strings, and provider-specific diagnostics are not public domain-contract fields.
- Pipe capture, child wait/reap, wall-clock timeout, retained-output overflow, and nonzero command exit remain distinct failure semantics; they must not be relabeled as spawn failures.
- No generic automatic retry is authorized by classification alone. A future retry policy requires its own causal evidence, bounded attempts/backoff, idempotency analysis, and lifecycle/cleanup proof.

## Classification contract

The minimum stable public classification after the RED executes is:

| Stable class | Rust/OS evidence | Meaning in this runtime |
| --- | --- | --- |
| `not_found` | `std::io::ErrorKind::NotFound`; POSIX `ENOENT` for executable/path resolution | The configured backend executable or a required path component could not be resolved. |
| `permission_denied` | `std::io::ErrorKind::PermissionDenied`; POSIX `EACCES` | The process lacked permission to execute or traverse the selected executable path. |
| `resource_exhausted` | `std::io::ErrorKind::WouldBlock` or `OutOfMemory` when returned by process creation/execution | Local resource pressure prevented the spawn attempt. This is diagnostic classification, not automatic retry authority. |
| `other` | Any other or future `ErrorKind` | Fail closed without leaking host-sensitive diagnostics. |

Rust `ErrorKind` is `#[non_exhaustive]` and its official documentation explicitly requires a wildcard arm for future variants. The implementation must therefore classify the few intentionally supported cases and map every remaining/current-future variant to `other`; an exhaustive copied list is not a stable API boundary.

POSIX.1-2024 independently distinguishes execution failures such as `ENOENT`, `EACCES`, `ENOEXEC`/`EINVAL`, and possible `ENOMEM`. This supports preserving causal categories rather than collapsing all failed execution attempts into one undifferentiated application error.

## Alternatives

1. Keep one `BackendInvocationFailed` value. Rejected: it prevents causal RCA and encourages speculative retry/workaround behavior.
2. Expose raw `io::Error`, errno, or error strings. Rejected: this leaks host/OS detail across the infrastructure ACL, creates unstable public contracts, and makes tests platform-sensitive.
3. Retry all spawn failures. Rejected: `NotFound` and `PermissionDenied` are not transient capacity failures, and retries can amplify pressure or duplicate side effects.
4. Preserve a bounded stable class while retaining the original typed timeout/output/capture/wait/nonzero-exit paths. Selected, contingent on exact-head RED and GREEN evidence.

## RED to GREEN acceptance

1. The deterministic missing-executable test executes and fails because the stable classification API/variant is absent, not because of formatting, fixture collision, runner setup, or another prerequisite.
2. The smallest production repair preserves `NotFound`, `PermissionDenied`, resource-exhausted (`WouldBlock`/`OutOfMemory`), and wildcard `Other` semantics.
3. Missing stdout/stderr capture remains a capture failure, not a spawn failure.
4. Timeout, output-limit, wait/reap, capture, and nonzero-exit regressions remain behaviorally distinct and fail closed.
5. No retry behavior is added.
6. Owned production rustdoc and statement/function/region/branch coverage remain 100%, with exact-head native/security evidence before normal integration.
7. Protected-head and release evidence are reacquired after integration; predecessor PR checks never transfer.

## References

The Open Group. (2024). *exec — execute a file*. In *The Open Group Base Specifications Issue 8, IEEE Std 1003.1-2024*. https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html

The Rust Project Developers. (2026). *ErrorKind in std::io* (Rust 1.98.1). https://doc.rust-lang.org/std/io/enum.ErrorKind.html
