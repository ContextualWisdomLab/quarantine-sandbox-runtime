# Command Result Time Authority Traceability

## Decision scope

`quarantine-sandbox-runtime` owns the execution evidence it emits. Consumer correlation data may identify a request, but a caller-supplied wall-clock value is not by itself an observed runtime fact.

At the #44 RED boundary, `RootlessPodmanAdapter::run_command_at` receives `started_at_epoch_seconds` from its caller, carries that value into command sandbox identity, and later copies the same value into `CommandExecutionResult.started_at_epoch_seconds`. `finished_at_epoch_seconds` is independently read from `SystemTime::now()` after cleanup. A caller can therefore supply a future start and obtain an otherwise-successful result whose completion precedes its start.

The security significance is evidence integrity and auditability, not container escape. A successful result must not present a contradictory chronology as if both timestamps were runtime observations.

## Evidence chain

| Evidence | Authority | Repository consequence |
| --- | --- | --- |
| `CommandExecutionRequest.request_id` | Consumer correlation | Remains opaque correlation metadata; it is not clock authority. |
| `started_at_epoch_seconds` method parameter | Current caller/test seam | May support deterministic tests, but must not be published unchanged as observed runtime time without validation or a reviewed clock abstraction. |
| `SystemTime::now()` at completion | Runtime wall clock | Supplies presentation/audit time only; it must not be used for lease enforcement arithmetic. |
| `Instant`/bounded runner deadlines | Runtime monotonic clock | Remains the correct class of source for elapsed-time timeout enforcement. |
| `CommandExecutionResult.started_at_epoch_seconds` / `finished_at_epoch_seconds` | Runtime-owned public evidence | Must be internally noncontradictory and must identify whether values are observed, supplied, or derived. |

## RED authority

Issue #44 is represented by `tests/podman_command_execution_timestamp_authority_red.rs`, first checked in at `ae38d9c9e594774f27943137e27a073b8743c2bc`.

The fixture keeps the current fake-Podman isolation path positive and supplies `1_000_000_000_000` as the caller start. Current production is expected to return `Ok` while copying that value into the result, so the test fails when it requires either fail-closed behavior or a runtime-observed start that is not the caller's impossible future value.

This RED is independent from:

- #35, which binds configured container timeout state;
- #43, which proves runtime-owned timeout termination;
- #34, which preserves output encoding integrity;
- #36, which binds lifecycle operations to acquired container identity.

## Smallest causal GREEN

Prefer an internal clock abstraction that can expose wall-clock observation for receipts and monotonic elapsed time for enforcement. Production should observe start and finish inside the runtime boundary; deterministic tests can inject a clock implementation rather than inject authoritative receipt timestamps through a public execution method.

A compatibility layer may temporarily retain `*_at` methods, but it must not silently convert contradictory caller input into plausible evidence. Rejecting an impossible supplied chronology is safer than clamping `finished_at` or copying the supplied value.

The repair must preserve existing timeout behavior, request correlation, sandbox identity ownership, exact-head provenance, and schema compatibility. If timestamp provenance becomes explicit on the wire, that is a versioned contract change rather than an undocumented semantic shift.

## Standards and research traceability

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Revision 5; Release 5.2.0 updates published 2025). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

AU-8 requires information systems to use system-generated timestamps for audit records. For this runtime, that supports keeping execution-evidence time authority inside the runtime rather than accepting unvalidated caller time as observed evidence.

Kent, K., & Souppaya, M. (2006). *Guide to computer security log management* (NIST Special Publication 800-92). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-92

SP 800-92 documents the analytic harm caused by inaccurate and inconsistent timestamps. The relevant implication here is narrower than distributed clock synchronization: one receipt must not contain a chronology the runtime itself can see is impossible.

## Release evidence

A future GREEN is not release evidence by source inspection alone. The #44 RED must first execute for the intended caller-time cause. The repaired exact head must then reacquire full repository CI/coverage/security/review and real rootless-Podman/positive-LSM evidence together with the existing #25–#43 gates.