# Backend Spawn Failure Traceability

## Decision status

Proposed on 2026-09-07. Issue #71 and Draft PR #72 own this focused observability repair above the runtime foundation. This document records stable causal evidence and design constraints only; transient queued/running job identifiers belong in PR/Issue/central owner-path metadata and are not versioned evidence.

## Problem and causal evidence

The original `src/infrastructure/bounded_command.rs` mapped every `std::process::Command::spawn()` failure to one `BoundedCommandError::Spawn` value and also used that same value when expected stdout/stderr pipes were absent. `src/infrastructure/podman.rs` then mapped `Spawn`, `Wait`, and `Capture` to `ApplicationServiceError::BackendInvocationFailed { operation }`. The resulting application-service error identified the failed operation but discarded the causal OS failure class.

Old exact native-CI specimens on #9 and #10 observed `BackendInvocationFailed { operation: "backend_security_info" }` before the downstream semantic assertion. Those observations did not establish whether the cause was a missing executable, permission denial, resource pressure, or another invocation failure, so they did not authorize a retry policy.

Draft #72 hardened its missing-executable RED by creating a test-owned empty directory and targeting an absent child path. Exact `5b623eceeb4b504da9042c9d1efd3b6c1b700fb9` then executed the intended causal RED: native verify passed exact checkout, dependency lock, repository policy, coverage-parser tests, and formatting before `cargo test` failed with E0432 because `BackendInvocationFailureKind` was not exported and E0599 because `ApplicationServiceError::BackendSpawnFailed` did not exist. The hosted negative rootless/AppArmor lane succeeded. This established the missing classification contract without relying on an ambient path or inferred failure.

After that RED, #72 adopted root checkout-credential repair `7482108c0b74f58f447722a98330f9ad44215eec` through an ordinary two-parent non-force merge. The minimum production candidate now preserves `std::io::ErrorKind` only at the infrastructure boundary, maps it into a bounded public vocabulary, separates missing captured pipes from process-spawn failure, and adds no retry behavior. Exact-head GREEN remains required before integration.

## DDD boundary

- `infrastructure::bounded_command` owns OS/process-spawn evidence and must preserve the typed `std::io::ErrorKind` from `Command::spawn()` until it reaches the backend ACL.
- `infrastructure::podman` owns translation from that OS evidence to the bounded backend-neutral failure class.
- `application_service` exposes only the small, stable failure vocabulary required for operability and controller decisions.
- Raw errno values, host paths, OS error strings, and provider-specific diagnostics are not public domain-contract fields.
- Pipe capture, child wait/reap, wall-clock timeout, retained-output overflow, and nonzero command exit remain distinct failure semantics; they must not be relabeled as spawn failures.
- No generic automatic retry is authorized by classification alone. A future retry policy requires its own causal evidence, bounded attempts/backoff, idempotency analysis, and lifecycle/cleanup proof.

## Classification contract

The minimum stable public classification is:

| Stable class | Rust/OS evidence | Meaning in this runtime |
| --- | --- | --- |
| `not_found` | `std::io::ErrorKind::NotFound`; POSIX `ENOENT` for executable/path resolution | The configured backend executable or a required path component could not be resolved. |
| `permission_denied` | `std::io::ErrorKind::PermissionDenied`; POSIX `EACCES` | The process lacked permission to execute or traverse the selected executable path. |
| `resource_exhausted` | `std::io::ErrorKind::WouldBlock` or `OutOfMemory` when returned by process creation/execution | Local resource pressure prevented the spawn attempt. This is diagnostic classification, not automatic retry authority. |
| `other` | Any other or future `ErrorKind` | Fail closed without leaking host-sensitive diagnostics. |

Rust `ErrorKind` is `#[non_exhaustive]` and its official documentation explicitly requires a wildcard arm for future variants. The implementation therefore classifies the intentionally supported cases and maps every remaining/current-future variant to `other`; an exhaustive copied list is not a stable API boundary.

POSIX.1-2024 independently distinguishes execution failures such as `ENOENT`, `EACCES`, `ENOEXEC`/`EINVAL`, and possible `ENOMEM`. This supports preserving causal categories rather than collapsing all failed execution attempts into one undifferentiated application error.

## Selected implementation

- `BoundedCommandError::Spawn` carries `std::io::ErrorKind` from the failed `Command::spawn()` call.
- Missing stdout/stderr handles are `Capture`, not `Spawn`.
- `BackendInvocationFailureKind` contains `NotFound`, `PermissionDenied`, `ResourceExhausted`, and `Other` only.
- `RootlessPodmanAdapter` maps `NotFound`, `PermissionDenied`, `WouldBlock`/`OutOfMemory`, and wildcard remaining/future `ErrorKind` values to that vocabulary.
- `ApplicationServiceError::BackendSpawnFailed` carries only the stable operation code and bounded failure class. `BackendInvocationFailed` remains the non-spawn path for wait/capture failures.
- Unit coverage exercises every explicit mapping and the wildcard branch; the real missing-executable integration RED remains the end-to-end `NotFound` acceptance.

## Alternatives

1. Keep one `BackendInvocationFailed` value. Rejected: it prevents causal RCA and encourages speculative retry/workaround behavior.
2. Expose raw `io::Error`, errno, or error strings. Rejected: this leaks host/OS detail across the infrastructure ACL, creates unstable public contracts, and makes tests platform-sensitive.
3. Retry all spawn failures. Rejected: `NotFound` and `PermissionDenied` are not transient capacity failures, and retries can amplify pressure or duplicate side effects.
4. Preserve a bounded stable class while retaining the original typed timeout/output/capture/wait/nonzero-exit paths. Selected after the causal RED executed.

## RED to GREEN acceptance

1. The deterministic missing-executable test executed and failed because the stable classification API/variant was absent, not because of formatting, fixture collision, runner setup, or another prerequisite. **Satisfied by the executed predecessor RED.**
2. The smallest production repair preserves `NotFound`, `PermissionDenied`, resource-exhausted (`WouldBlock`/`OutOfMemory`), and wildcard `Other` semantics. **Implemented; exact-head verification pending.**
3. Missing stdout/stderr capture remains a capture failure, not a spawn failure. **Implemented; exact-head verification pending.**
4. Timeout, output-limit, wait/reap, capture, and nonzero-exit regressions remain behaviorally distinct and fail closed. **Exact-head verification pending.**
5. No retry behavior is added. **Implemented.**
6. Owned production rustdoc and statement/function/region/branch coverage remain 100%, with exact-head native/security evidence before normal integration. **Pending.**
7. Protected-head and release evidence are reacquired after integration; predecessor PR checks never transfer. **Pending.**

## References

The Open Group. (2024). *exec — execute a file*. In *The Open Group Base Specifications Issue 8, IEEE Std 1003.1-2024*. https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html

The Rust Project Developers. (2026). *ErrorKind in std::io* (Rust 1.98.1). https://doc.rust-lang.org/std/io/enum.ErrorKind.html
