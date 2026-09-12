# Application-service create receipt authority traceability

## Scope

This note records the application-service `podman create` identity/cleanup authority repair owned by Draft PR #21 and P0 issue #113. It does not promote the mutable PR head to release authority and does not substitute fake-backend evidence for positive runtime confinement.

## Problem and invariant

A successful `podman create` may have created a container even when stdout is malformed or unavailable as an admissible backend identifier. The generated `qsr-app-*` correlation name is not sufficient destructive authority: cleanup must use an exact backend identity that the current invocation can prove it owns.

The application-service adapter therefore treats a runtime-owned `--cidfile` receipt as a separate authority channel. Container cleanup may target only an admitted exact identifier. Generated correlation names remain audit/public lease metadata and must never be promoted to `rm --force` authority.

## Implemented boundary

Current PR #21 lineage provisions a private temporary create receipt, inserts `--cidfile=<runtime-owned path>` into the create argv, and reconciles stdout with the receipt before post-create lifecycle operations.

The admission contract is intentionally narrower for destructive receipt authority than the legacy stdout parser: a receipt is one exact 64-character lower-case hexadecimal identifier. Missing, malformed, invalid UTF-8, mismatched, or unreadable receipt state fails closed. When an exact receipt or already-admitted stdout identifier exists, cleanup uses only that exact identifier. Cleanup failure takes precedence over the original create/reconciliation error because incomplete cleanup is the stronger operational failure.

No branch falls back to `rm --force qsr-app-*`.

## Causal and edge evidence

- `4a94e20264dc8aaf1d2f71d70a41ffd69bd2aa65` moved the application-service process fixture to the checked-in immutable fake-Podman executable plus data-only sidecars, preserving the process-boundary RCA established by `BACKEND_SPAWN_FAILURE_TRACEABILITY.md`.
- `5504c5a2e384dca08d7fb8bbeca8cb329e263647` applied the same immutable-fixture discipline to create-receipt edge cases.
- `d8c17c78d8d3164d4290290dc0e6c903acb75ec7` added real OS-boundary receipt failures: receipt read I/O failure after failed create, temporary-directory creation failure, and non-UTF-8 receipt path rejection before resource creation.
- `f28d9ac2fc8dd404b6b297478e135f73e0383b4e` added cleanup-precedence cases covering failed create, malformed stdout, receipt mismatch/read failure, exact-ID container cleanup failure, and network cleanup failure while forbidding generated-name destructive fallback.
- Exact CI `34667414544` on `f28d9ac2...` passed verify and hosted negative rootless/AppArmor. Production coverage reached lines `2072/2072` and functions `202/202`; branch coverage reached `460/460`. The canonical source-region gate retained one uncovered region at the `plan_at` propagation of `runtime_identity()` failure, so 100% coverage was not claimed.
- `c41f721e3044c44fca7e1e1350bb48c935429436` introduced a private identity-source seam that preserves public `plan_at` behavior and validation/expiry ordering while allowing the existing typed `RuntimeIdentityUnavailable` path to be exercised through launch planning. The purpose-bound source-fix workflow removed itself in the same ordinary descendant after full tests, Clippy `-D warnings`, rustdoc `-D warnings`, formatting, and `git diff --check` passed.

## Evidence hierarchy and remaining gates

The fake-Podman suite proves authority selection, error precedence, cleanup targeting, parser/ACL behavior, and lifecycle call ordering. It does not prove rootless Podman, cgroup, tmpfs, seccomp, AppArmor/SELinux, or network enforcement on a live sandbox.

Issue #113 remains open until the exact current candidate is independently reviewed, inherits the stabilized canonical foundation through ordinary non-force integration, obtains the dedicated positive effective-LSM/runtime evidence, and reaches a protected integrated head. No version, tag, package, GitHub Release, consumer pin, or release claim is authorized by this note.
