# Backend spawn failure traceability

## Scope and owner boundary

This note records the process-boundary repair integrated on the active network-owner ancestry. `sandbox_execution` remains the Core bounded context; `application_service` remains a Supporting context; direct process execution and Podman error translation remain infrastructure concerns. The public contract exposes a bounded failure vocabulary rather than raw errno values, host paths, provider diagnostics, or retry policy.

## Executed RED

Exact `76d6d2ee6423f871967245e299411c4199ff875a` executed in native CI run `34902839308`. Verify job `104172571319` checked out that exact SHA, passed dependency lock validation, repository policy, coverage-parser tests, and `cargo fmt --check`, then failed during `cargo test --locked --workspace --all-targets` in `tests/backend_spawn_failure_classification_red.rs`.

The compiler produced the intended causal RED:

- `E0432`: `BackendInvocationFailureKind` was not exported by the crate root.
- `E0599`: `ApplicationServiceError` had no `BackendSpawnFailed` variant.

The hosted negative rootless/AppArmor job `104172571122` completed successfully on the same exact head. Positive SELinux remained a separate self-hosted lane and is not substituted by hosted negative evidence.

## Minimum causal repair

Production candidate `54fb9e33fbafd2b2549d95e4691464749e2dbf63` adapts only the small typed-spawn contract from canonical process owner #72 into the current #119 ancestry:

1. `BoundedCommandError::Spawn(io::ErrorKind)` preserves the OS-derived failure category at the process boundary.
2. Missing output pipes are classified as `Capture`, not `Spawn`; `Wait`, `Timeout`, `OutputLimit`, and `Capture` remain distinct internal classes.
3. `application_service` exposes `BackendInvocationFailureKind::{NotFound, PermissionDenied, ResourceExhausted, Other}` and `BackendSpawnFailed { operation, failure_kind }` without raw errno/path/provider strings.
4. The Podman infrastructure ACL maps `NotFound`, `PermissionDenied`, `WouldBlock`/`OutOfMemory`, and all other current/future `ErrorKind` values to the bounded public vocabulary; `Wait`/`Capture` continue to map to generic post-spawn invocation failure.
5. Generic invocation display text is provider-neutral. No retry, sleep, mutex, timeout broadening, or isolation weakening is introduced.

The wildcard `Other` mapping is deliberate because Rust marks `std::io::ErrorKind` as `#[non_exhaustive]` and explicitly requires callers to remain forward-compatible with future variants. POSIX.1-2024 separately distinguishes execution failures such as `ENOENT`, `EACCES`, and possible `ENOMEM`; the public vocabulary preserves operationally useful classes without coupling the domain contract to platform errno integers.

## Verification gate

This repair is not GREEN merely because the source candidate exists. The authoritative GREEN requires a fresh exact-head run after this traceability commit with repository policy, formatting, full tests, Clippy `-D warnings`, rustdoc `-D warnings`, complete owned-production coverage, branch coverage, hosted negative confinement, and the repository's applicable security/review gates. Dedicated positive-LSM evidence remains independently required for any real-isolation or release claim.

After process-boundary GREEN, the intended sequence is #119 inherited network RED replay, #120 explicit-termination foreign-member RED, #122 acquired-network-identity RED, and only then the minimum network lifecycle production repair.

## References

The Open Group. (2024). *exec — execute a file*. In *The Open Group Base Specifications Issue 8, IEEE Std 1003.1-2024*. https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html

The Rust Project Developers. (2026). *ErrorKind in std::io*. Rust standard library documentation. https://doc.rust-lang.org/stable/std/io/enum.ErrorKind.html
