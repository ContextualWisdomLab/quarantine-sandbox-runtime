# Application-service runtime identity traceability

Status: issue #20 has a runner-backed causal RED on exact `368e20eeeb0ac3d573a913af981dcb5dd4104b1a`; the minimum runtime-identity repair is under exact-head verification on Draft #21.

## Problem and bounded-context ownership

`application_service` owns consumer request/correlation semantics. `infrastructure` owns concrete Podman resource selection, and `sandbox_execution` remains the Core authority for reusable runtime lifecycle semantics. A consumer `request_id` is therefore not a globally unique Podman ownership identifier and must not become a destructive resource capability.

Before this repair, `RootlessPodmanAdapter::plan_at` derived the `qsr-app-*` and `qsr-net-*` suffix from SHA-256 over `(request_id, image_reference, policy_id, started_at_epoch_seconds)`, truncated to 16 hexadecimal characters. Two independent runtime instances receiving the same request under the same policy in the same whole second consequently generated the same cleanup-owning container and network names.

## Exact causal RED

Native CI `34301336763`, verify job `102308701305`, checked out exact `368e20eeeb0ac3d573a913af981dcb5dd4104b1a`. Dependency lock, repository policy, coverage-parser tests, rustfmt, the existing application-service suite, create-ID admission regressions, and the forged-lease authority regression all passed before `tests/podman_application_service_identity_race_red.rs` executed.

The concurrent same-request/same-second test then failed at the intended ownership invariant:

- first sandbox identity: `qsr-app-2500e69e869f94b7`;
- second sandbox identity: `qsr-app-2500e69e869f94b7`;
- assertion: independent launches must never share a cleanup-owning sandbox identity.

This is a semantic RED rather than a formatter, dependency, repository-policy, or fake-backend prerequisite failure.

## Decision

Each launch obtains 128 bits directly from the operating system random source through pinned `getrandom` 0.4.3 and encodes all 16 bytes as 32 lowercase hexadecimal characters. The same invocation suffix is used for the generated container name, generated network name, and runtime identity label. The consumer `request_id` remains unchanged correlation/idempotency metadata.

Entropy acquisition is fail-closed. `getrandom::fill` failure maps to the typed `ApplicationServiceError::RuntimeIdentityUnavailable`; a failed entropy read does not fall back to request-derived identity, timestamps, process-local counters, or mutable resource lookup.

The helper boundary is injectable for deterministic unit evidence: one test injects a fixed 16-byte value and proves stable lower-hex encoding, and one injects an entropy failure and proves the typed error. Runtime production still calls the operating-system source directly.

This choice is collision-resistance evidence, not bearer authorization. RFC 9562 recommends CSPRNG-backed random values when low collision probability and unguessability are required, while also warning that identifiers must not be treated as security capabilities. Destructive container authority remains separately owned by issue #40, which requires the exact long container ID acquired from successful `podman create` for post-create lifecycle operations. Issue #42 separately prevents serialized lease evidence from recreating cleanup authority.

## Rejected alternatives

- Keep deterministic `(request, policy, whole-second)` hashing: rejected by exact CI because independent runtimes produced an identical resource identity.
- Require callers to make `request_id` globally unique: rejected because it changes consumer correlation/idempotency semantics and moves infrastructure ownership into the caller contract.
- Add only a process-local counter or mutex: rejected because it does not cover independent processes, restarts, or stale resources.
- Add PID/time to the deterministic hash: rejected because reuse/restart and coarse-time aliasing remain deployment-dependent rather than providing a runtime-owned collision-resistant identity.
- Use a longer generated name but keep name-based post-create destructive selection: rejected as a #40 substitute. Collision-resistant correlation names do not turn mutable Podman names into immutable acquired-resource authority.
- Retry entropy acquisition with a deterministic fallback: rejected because it hides an ownership-evidence failure and can recreate the original collision class.

## Code/test linkage

| Evidence | Exact responsibility |
| --- | --- |
| `src/infrastructure/podman.rs::runtime_identity` | Production OS-entropy acquisition and lower-hex encoding. |
| `src/infrastructure/podman.rs::RootlessPodmanAdapter::plan_at` | Binds one fresh invocation suffix into container name, network name, and identity label after request/policy validation. |
| `src/application_service/mod.rs::ApplicationServiceError::RuntimeIdentityUnavailable` | Stable fail-closed error when runtime identity entropy cannot be obtained. |
| `tests/podman_application_service_identity_race_red.rs` | Concurrent same-request/same-second ownership regression; remains compatible with #40 by not requiring destructive selectors to equal public correlation names. |
| `tests/application_service_validation.rs` | Proves same consumer inputs create distinct runtime plans while preserving name shape, same-plan container/network suffix binding, validation, and expiry behavior. |
| Issue #20 / Draft #21 | Decision and integration authority. |

Candidate lineage begins with `850ace94f578e53329a8b9f2465542f3224f8f48` (typed entropy failure) and `896e25db4753193cc43a7ff319a47efb0cfe0da5` (128-bit runtime identity), followed by test-contract repairs `7b6d9790b51d6d08149597d6a6f45bca79846ef0` and `fd342b1abdb1fc24761b06670bfed8b75d56ac55`. These commits are candidates only until a fresh exact head passes the required hosted gates; positive effective-LSM and protected integration remain independent release gates.

## References

Davis, K. R., Peabody, B. G., & Leach, P. J. (2024). *Universally unique IDentifiers (UUIDs)* (RFC 9562). RFC Editor. https://doi.org/10.17487/RFC9562

Rust Random Project. (2026). *getrandom 0.4.3: System's random number generator*. Docs.rs. https://docs.rs/getrandom/0.4.3/getrandom/

Rust Random Project. (2026). *getrandom::fill*. Docs.rs. https://docs.rs/getrandom/0.4.3/getrandom/fn.fill.html
