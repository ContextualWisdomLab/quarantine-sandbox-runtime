# Backend Spawn Failure Traceability

Status: Proposed on canonical command-runtime owner PR #14. This record is evidence for the active branch only; it is not release authority until the unchanged integrated protected head satisfies every release gate.

## Problem and causal evidence

The command-runtime process boundary previously collapsed process creation, child observation/reaping, and output-pipe capture into `ApplicationServiceError::BackendInvocationFailed`. That erased the distinction between a backend executable that cannot be created at all and a backend process that was created but later could not be supervised. The loss of cause was visible in broad hosted jobs as rotating `BackendInvocationFailed { operation: "backend_security_info" }` specimens, so leaf tests could not distinguish a deterministic product defect from a process-boundary failure.

The owner-path RED was committed at `102dbf7ead72a4a0d7e87f6a51684ba5c917bee2`. Native CI `34327610116` reached the new regression in branch coverage and failed to compile because `BackendInvocationFailureKind` and `ApplicationServiceError::BackendSpawnFailed` did not exist. Verify independently exposed a formatting defect in the newly added test before execution; test-only `d966505e7519b6708132ae30840f4f4e67330665` repaired formatting and changed the expected operation from the older #72 name `rootless_probe` to the current owner operation `backend_security_info`. This preserves the counterexample while adapting it to the live owner instead of copying the older #72 tree.

## DDD owner and invariant

`application_service` owns the stable consumer-visible backend error contract. `infrastructure::bounded_command` owns OS process lifecycle facts. `infrastructure::podman` is the ACL that translates OS-specific process facts into the bounded application-service vocabulary.

Invariant: a failure before process creation must remain distinguishable from failures after process creation, but raw errno values, executable paths, provider-specific strings, and unbounded platform detail must not cross the application-service boundary.

The public vocabulary is therefore limited to `BackendInvocationFailureKind::{NotFound, PermissionDenied, ResourceExhausted, Other}`. `std::io::ErrorKind` remains inside the infrastructure boundary. `WouldBlock` and `OutOfMemory` map to `ResourceExhausted`; unknown and future non-exhaustive variants map to `Other`. Wait/reap and pipe-capture failures remain `BackendInvocationFailed`, while timeout and output-limit contracts remain unchanged.

## Decision and implementation

Chosen repair:

- `50644233eeea8c9f3a1a4843ea999d863c3f6249` preserves `io::ErrorKind` only on `BoundedCommandError::Spawn` and classifies absent stdout/stderr pipes as `Capture`, not `Spawn`.
- `d98127705cf3f1221f2b3e3641d4595e32aaf14a` adds the bounded public failure kind and `BackendSpawnFailed { operation, failure_kind }`; the remaining generic invocation error text is provider-neutral.
- `db7f1d9d6477c8ffef0899f73f6f13ab64ff45f5` exports the bounded public enum.
- `6d7874409e20975d9db44041210670dc715d7e25` translates actual Spawn failures in the Podman ACL and adds unit coverage for every public class plus generic Wait/Capture.
- `b36f6c6925119d1c65d41d5d08841541ad8f2762` restores non-obvious lifecycle, isolation, cleanup, and evidence comments that were accidentally lost during the complete-file connector write. No intended runtime behavior was changed by that correction.

Rejected alternatives:

- Retry/sleep/mutex: hides the process failure and changes timing without identifying its cause.
- Raw errno or `io::ErrorKind` in the public API: leaks platform detail and couples consumers to an explicitly non-exhaustive Rust enum.
- Treat missing output pipes as Spawn: a child already exists, so this falsifies lifecycle phase.
- Copy or merge the old #72 tree wholesale: it predates multiple #14 lifecycle, ENTRYPOINT, image-integrity, and cleanup-ownership repairs and would violate the canonical owner/single-writer boundary.

## CI credential boundary discovered during the same owner sweep

The exact #14 workflow still used the default checkout credential persistence, while the integration-root contract had already disabled it. The prior exact job logs showed checkout configuring persisted credentials. Test-only `ba0826c6b4739a3cafb5c955fac00688a690ded4` adopts the root regression `every_checkout_discards_persisted_credentials`; `4b6f7ff121dfdd671decfafd45413b5c7bc4814d` applies `persist-credentials: false` to all five read-only checkout steps. This is CI security hardening and does not alter production runtime semantics.

## Evidence linkage and release gate

Primary files and APIs: `src/infrastructure/bounded_command.rs`, `src/infrastructure/podman.rs`, `src/application_service/mod.rs`, `src/lib.rs`, and `tests/backend_spawn_failure_classification_red.rs`. The final exact branch head must newly pass repository policy, rustfmt, full Rust tests, Clippy, rustdoc, complete owned-production statement/function/region/branch coverage, hosted negative confinement, dedicated positive effective-LSM, qualifying review, and central security/dependency gates. Predecessor GREEN does not transfer.

The typed discriminator is diagnostic authority, not proof that every previously rotating `backend_security_info` specimen was a Spawn failure. When those tests re-run on an unchanged head, `BackendSpawnFailed` authorizes Spawn-specific RCA; a remaining generic error narrows RCA to Wait/Capture. No errno may be inferred before that evidence exists.

## References

Rust Project. (2026). *ErrorKind in std::io*. The Rust Standard Library. https://doc.rust-lang.org/std/io/enum.ErrorKind.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

GitHub. (2026). *GitHub repository checkout: Git credentials after checkout*. GitHub Agentic Workflows. https://github.github.com/gh-aw/reference/checkout/
