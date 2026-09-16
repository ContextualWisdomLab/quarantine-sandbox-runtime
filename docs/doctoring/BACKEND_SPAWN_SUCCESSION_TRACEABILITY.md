# Backend spawn succession traceability

## Problem and owner boundary

The canonical application-service network successor must preserve the bounded process-spawn failure contract already proved on historical network/process integration work. The pre-repair network-owner source collapsed `BoundedCommandError::Spawn`, `Wait`, and `Capture` into the same public `BackendInvocationFailed` surface and its generic message named Podman directly. That lost the causal distinction between failure to create the backend process and failure after process creation.

`sandbox_execution` owns reusable bounded execution semantics. `infrastructure` owns operating-system process observation and backend ACL translation. `application_service` owns the stable consumer-visible error vocabulary. Raw errno values, host paths, provider diagnostics, retry policy, and process implementation details do not cross that boundary.

## Historical causal evidence retained

Historical #124 predecessor exact `76d6d2ee6423f871967245e299411c4199ff875a`, CI `34902839308`, executed the typed-spawn RED after dependency, repository, coverage-policy, and formatting gates. The intended compile-time failures were the absence of public `BackendInvocationFailureKind` and `ApplicationServiceError::BackendSpawnFailed`. Hosted negative rootless/AppArmor passed on the same exact head.

#124 later adapted the minimum typed-spawn contract and, on exact `6c653d36f16b18af353949e0f0152c8527f48396`, the bounded-command and backend-spawn classification tests passed before that head reached the independent application-service network-binding RED. That remains valid predecessor evidence and delta; it was not treated as current #127 GREEN.

## Current-owner causal execution

Current-owner test `tests/backend_spawn_succession_owner_red.rs` replayed only the contract required for safe succession:

- a definitely missing backend executable must surface `BackendSpawnFailed { operation: "rootless_probe", failure_kind: NotFound }`;
- generic post-spawn invocation text must remain provider-neutral.

Exact predecessor `7b2b71f2557ab510dee3dfa0a79a9e1ebbda6185`, CI `35083728065`, executed the witness. Coverage job `104753533940` reached compilation and failed for the intended first cause: unresolved public `BackendInvocationFailureKind` and missing `ApplicationServiceError::BackendSpawnFailed`. The same exact head's verify job `104753534176` passed repository validation and the Python coverage-policy suite, then independently exposed rustfmt drift in current network-owner source/tests. These are separate findings; formatter drift is not evidence for or against the typed-spawn contract.

## Minimum causal repair

Production descendant `a442f343b802127f5d646155934fb736ce0e9099` adapts the reviewed semantic delta onto current #127 ancestry rather than replacing the older branch wholesale:

1. `BoundedCommandError::Spawn(io::ErrorKind)` preserves `io::Error::kind()` at the actual spawn boundary;
2. missing output pipes are `Capture`, while `Wait`, `Timeout`, `OutputLimit`, and `Capture` remain distinct from spawn;
3. public `BackendInvocationFailureKind::{NotFound, PermissionDenied, ResourceExhausted, Other}` plus `BackendSpawnFailed` expose a bounded class without raw errno or host-path disclosure;
4. `NotFound` and `PermissionDenied` map directly, `WouldBlock | OutOfMemory` map to `ResourceExhausted`, and the wildcard maps future/non-specialized spawn kinds to `Other`;
5. generic post-spawn invocation text is provider-neutral;
6. the stale missing-executable process-boundary expectation now requires `BackendSpawnFailed { failure_kind: NotFound }`;
7. rustfmt-only drift reported by exact predecessor verify was repaired without changing network semantics.

Every current network identity, cleanup-authority, attachment, evidence-admission, and pre-attestation RED remains in place. Retry, sleep, mutex serialization, timeout broadening, provider-specific generic text, raw errno exposure, force push, and destructive rebase remain rejected.

## Current verification and succession gate

`a442f343b802127f5d646155934fb736ce0e9099` is a candidate causal repair, not an exact-head GREEN claim. Its fresh CI is `35115443488` and must independently execute the current-owner spawn witness plus repository validation, rustfmt, locked workspace/all-target tests, Clippy `-D warnings`, public/private rustdoc `-D warnings`, complete owned-production statement/function/region/branch and edge coverage, hosted negative evidence, applicable positive effective-LSM evidence, and qualifying review/security gates.

#124 remains open until this successor proves the typed-spawn contract and its remaining valid tests/evidence, including historical effective network-attachment findings, are demonstrably mapped into verified current-owner work.

## References

The Rust Project Developers. (2026). *ErrorKind in std::io*. Rust standard library documentation. https://doc.rust-lang.org/stable/std/io/enum.ErrorKind.html

The Rust Project Developers. (2026). *Error in std::io*. Rust standard library documentation. https://doc.rust-lang.org/std/io/struct.Error.html

The Open Group. (2024). *exec — execute a file*. In *The Open Group Base Specifications Issue 8, IEEE Std 1003.1-2024*. https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html
