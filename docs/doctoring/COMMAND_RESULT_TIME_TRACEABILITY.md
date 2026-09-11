# Command Result Time Authority Traceability

## Decision scope

`quarantine-sandbox-runtime` owns the execution evidence it emits. Consumer correlation data may identify a request, but a caller-supplied wall-clock value is not by itself an observed runtime fact.

At the #44 RED boundary, `RootlessPodmanAdapter::run_command_at` receives `started_at_epoch_seconds` from its caller, carries that value into command sandbox identity, and later copies the same value into `CommandExecutionResult.started_at_epoch_seconds`. `finished_at_epoch_seconds` is independently read from `SystemTime::now()` after cleanup. A caller can therefore supply a future start and obtain an otherwise-successful result whose completion precedes its start.

The security significance is evidence integrity and auditability, not container escape. A successful result must not present a contradictory chronology as if both timestamps were runtime observations.

## Evidence chain

| Evidence | Authority | Repository consequence |
| --- | --- | --- |
| `CommandExecutionRequest.request_id` | Consumer correlation | Remains opaque correlation metadata; it is not clock authority. |
| `started_at_epoch_seconds` method parameter | Compatibility/test seam | May support deterministic tests, but is not runtime-observed receipt authority. |
| Runtime `SystemTime::now()` observations | Runtime wall clock | Supply presentation/audit timestamps and must fail closed on pre-epoch or contradictory chronology. |
| `Instant`/bounded runner deadlines | Runtime monotonic clock | Remains the correct class of source for elapsed-time timeout enforcement. |
| `CommandExecutionResult.started_at_epoch_seconds` / `finished_at_epoch_seconds` | Runtime-owned public evidence | Must be internally noncontradictory and originate inside the runtime boundary. |

## RED authority

Issue #44 is represented by `tests/podman_command_execution_timestamp_authority_red.rs`, first checked in at `ae38d9c9e594774f27943137e27a073b8743c2bc`.

The fixture keeps the current fake-Podman isolation path positive and supplies `1_000_000_000_000` as the caller start. The repaired witness first proves that the same fake runtime can construct a normal result, then drives the future caller timestamp through live isolation, wait/log collection and exact-ID cleanup before testing chronology. Draft #111 exact `d45ec905eb55bb2b23737995e96baf698cf4de16`, CI `34565196344`, failed at that intended assertion with the caller value still published unchanged. That is the causal RED.

This RED is independent from:

- #35, which binds configured container timeout state;
- #43, which proves runtime-owned timeout termination;
- #34, which preserves output encoding integrity;
- #36, which binds lifecycle operations to acquired container identity.

## Selected causal repair

The canonical repair keeps the existing `*_at` parameter only as a compatibility/test seam but removes it from evidence authority. `run_command_with_binding_at` observes start and finish with `SystemTime::now()` inside the runtime boundary, uses the observed start for sandbox identity and result evidence, and fails closed when either observation predates the Unix epoch or the finish observation precedes the start observation. It does not clamp, synthesize, or reorder timestamps. Existing `Instant`/`BoundedCommandRunner` deadlines remain the monotonic authority for lease and administrative timeout enforcement.

Focused deterministic unit tests cover equal/ordered observations, a wall-clock rollback, pre-epoch failure, and exact epoch conversion. The executed #111 future-input witness verifies that the caller value is no longer emitted unchanged by the repaired implementation.

## Exact validation evidence

Draft #112 exact `f811e37b230dd836da8b6b44c43d729d55815341`, CI `34569963535`, completed exact checkout, dependency lock, repository/CI-contract validation, rustfmt, `cargo test --locked --workspace --all-targets --no-fail-fast`, Clippy with `-D warnings`, rustdoc with `-D warnings`, and the hosted negative rootless/AppArmor lane successfully. Coverage evidence generation also completed; only the explicit repository-wide 100% admission checks failed. Measured coverage was lines `4691/4842 = 96.88%`, functions `435/446 = 97.53%`, regions `6302/6542 = 96.33%`, branches `657/728 = 90.25%`.

A subsequent one-shot repair on exact `9700e1ec8f3d40d04ed9c8485d27da188e4322ec` removed an unreachable registry-split control-flow branch after running the focused malformed-image regression, full workspace tests, rustfmt, Clippy, rustdoc, and `git diff --check`. Run `34573368198`, job `103180204743`, completed successfully and published ordinary descendant `9a3bc933328cf90ad834bcd532ad13ac5bfc967b`; the workflow removed itself in the same commit. Because a `GITHUB_TOKEN`-originated push does not provide independent exact-head CI evidence for the descendant, this document update intentionally creates a normal owner-authored descendant so current-head CI can re-establish exact-SHA evidence rather than inheriting predecessor GREEN by assumption.

## Standards and research traceability

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Revision 5; Release 5.2.0 updates published 2025). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

AU-8 requires information systems to use system-generated timestamps for audit records. For this runtime, that supports keeping execution-evidence time authority inside the runtime rather than accepting unvalidated caller time as observed evidence.

Kent, K., & Souppaya, M. (2006). *Guide to computer security log management* (NIST Special Publication 800-92). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-92

The citation follows the final CSRC publication record and contemporaneous NIST material, which identify the 2006 author as Karen Kent. A separate NIST publications-catalog page currently renders the same DOI under Karen A. Scarfone; that later-name metadata alias is not used to rewrite the author name printed for the 2006 final publication.

SP 800-92 documents the analytic harm caused by inaccurate and inconsistent timestamps. The relevant implication here is narrower than distributed clock synchronization: one receipt must not contain a chronology the runtime itself can see is impossible.

## Release evidence

The causal RED and functional repair are established, but they are not sufficient release evidence by themselves. The repaired current head must reacquire exact-head repository CI and 100% owned-production line/function/region/branch coverage, independent review/security evidence, real rootless-Podman enforcement for #35/#43, positive effective-LSM evidence, protected-head verification, SBOM/provenance/reproducibility/rollback, and immutable publication before the command chronology repair can be called released.
