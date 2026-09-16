# Backend spawn succession traceability

## Problem and owner boundary

The canonical application-service network successor must preserve the bounded process-spawn failure contract already proved on historical network/process integration work. Current network-owner source still collapses `BoundedCommandError::Spawn`, `Wait`, and `Capture` into the same public `BackendInvocationFailed` surface and its generic message names Podman directly. That loses the causal distinction between failure to create the backend process and failure after process creation.

`sandbox_execution` owns reusable bounded execution semantics. `infrastructure` owns operating-system process observation and backend ACL translation. `application_service` owns the stable consumer-visible error vocabulary. Raw errno values, host paths, provider diagnostics, retry policy, and process implementation details do not cross that boundary.

## Historical causal evidence retained

Historical #124 predecessor exact `76d6d2ee6423f871967245e299411c4199ff875a`, CI `34902839308`, executed the typed-spawn RED after dependency, repository, coverage-policy, and formatting gates. The intended compile-time failures were the absence of public `BackendInvocationFailureKind` and `ApplicationServiceError::BackendSpawnFailed`. Hosted negative rootless/AppArmor passed on the same exact head.

#124 later adapted the minimum typed-spawn contract and, on exact `6c653d36f16b18af353949e0f0152c8527f48396`, the bounded-command and backend-spawn classification tests passed before that head reached the independent application-service network-binding RED. That is valid predecessor evidence and delta, but it is not current #127 exact-head GREEN.

## Current-owner RED

Current-owner test `tests/backend_spawn_succession_owner_red.rs` replays only the contract required for safe succession:

- a definitely missing backend executable must surface `BackendSpawnFailed { operation: "rootless_probe", failure_kind: NotFound }`;
- generic post-spawn invocation text must remain provider-neutral.

The test intentionally depends on the bounded public vocabulary that current #127 does not yet expose. Until an exact current head executes this witness, no production adaptation is admitted and no historical GREEN is transferred.

## Minimum later repair boundary

After current-owner causal execution, the smallest admissible repair is the already-reviewed semantic shape, adapted onto current network ancestry rather than copied wholesale from the older branch:

1. preserve `io::Error::kind()` at the spawn boundary as `BoundedCommandError::Spawn(io::ErrorKind)`;
2. keep missing output pipes as `Capture`, and keep `Wait`, `Timeout`, `OutputLimit`, and `Capture` distinct from spawn;
3. expose bounded `BackendInvocationFailureKind::{NotFound, PermissionDenied, ResourceExhausted, Other}` plus `BackendSpawnFailed` without raw errno or host-path disclosure;
4. map expected `ErrorKind` values deliberately and retain a wildcard-compatible public `Other` class for future variants;
5. keep generic post-spawn invocation failures provider-neutral;
6. preserve every current #127 network identity, cleanup-authority, attachment, evidence-admission, and pre-attestation RED unchanged.

Retry, sleep, mutex serialization, timeout broadening, provider-specific public text, raw errno exposure, sibling-branch source replacement, force push, and destructive rebase are rejected alternatives.

## Verification and succession gate

A repair is not successor completion until one unchanged #127 descendant passes repository validation, rustfmt, locked workspace/all-target tests, Clippy `-D warnings`, public/private rustdoc `-D warnings`, complete owned-production statement/function/region/branch and edge coverage, hosted negative evidence, applicable positive effective-LSM evidence, and qualifying review/security gates. #124 remains open until its valid typed-spawn delta/tests/contract/TRACEABILITY and network execution evidence are demonstrably mapped into verified successor work.

## References

The Rust Project Developers. (2026). *ErrorKind in std::io*. Rust standard library documentation. https://doc.rust-lang.org/stable/std/io/enum.ErrorKind.html

The Rust Project Developers. (2026). *Error in std::io*. Rust standard library documentation. https://doc.rust-lang.org/std/io/struct.Error.html

The Open Group. (2024). *exec — execute a file*. In *The Open Group Base Specifications Issue 8, IEEE Std 1003.1-2024*. https://pubs.opengroup.org/onlinepubs/9799919799/functions/exec.html
