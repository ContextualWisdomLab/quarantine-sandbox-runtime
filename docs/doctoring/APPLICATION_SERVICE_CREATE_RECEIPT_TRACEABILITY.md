# Application-service create receipt authority traceability

## Scope

This note records the application-service `podman create` identity/cleanup authority repair owned by Draft PR #21 and P0 issue #113. It does not promote the mutable PR head to release authority and does not substitute fake-backend evidence for positive runtime confinement.

## Problem and invariant

A successful `podman create` may have created a container even when stdout is malformed or unavailable as an admissible backend identifier. The generated `qsr-app-*` correlation name is not sufficient destructive authority: cleanup must use an exact backend identity that the current invocation can prove it owns.

The application-service adapter therefore treats a runtime-owned `--cidfile` receipt as a separate authority channel. Container cleanup may target only an admitted exact identifier. Generated correlation names remain audit/public lease metadata and must never be promoted to `rm --force` authority.

The stdout and receipt proof channels must also enforce the same identifier grammar before destructive use. The repository contract is one exact 64-character lower-case hexadecimal container identifier. Upper-case `A-F` is not normalized and must not be admitted as an alternative spelling of destructive authority.

## Implemented boundary

Current PR #21 lineage provisions a private temporary create receipt, inserts `--cidfile=<runtime-owned path>` into the create argv, and reconciles stdout with the receipt before post-create lifecycle operations.

The admission contract is intentionally narrow for destructive authority: stdout and receipt evidence must resolve to one exact 64-character lower-case hexadecimal identifier. Missing, malformed, invalid UTF-8, mismatched, unreadable, name-like, short, non-hexadecimal, upper-case, or padded identity evidence fails closed. When an exact receipt or already-admitted stdout identifier exists, cleanup uses only that exact identifier. Cleanup failure takes precedence over the original create/reconciliation error because incomplete cleanup is the stronger operational failure.

No branch falls back to `rm --force qsr-app-*`.

## Causal and edge evidence

- `4a94e20264dc8aaf1d2f71d70a41ffd69bd2aa65` moved the application-service process fixture to the checked-in immutable fake-Podman executable plus data-only sidecars, preserving the process-boundary RCA established by `BACKEND_SPAWN_FAILURE_TRACEABILITY.md`.
- `5504c5a2e384dca08d7fb8bbeca8cb329e263647` applied the same immutable-fixture discipline to create-receipt edge cases.
- `d8c17c78d8d3164d4290290dc0e6c903acb75ec7` added real OS-boundary receipt failures: receipt read I/O failure after failed create, temporary-directory creation failure, and non-UTF-8 receipt path rejection before resource creation.
- `f28d9ac2fc8dd404b6b297478e135f73e0383b4e` added cleanup-precedence cases covering failed create, malformed stdout, receipt mismatch/read failure, exact-ID container cleanup failure, and network cleanup failure while forbidding generated-name destructive fallback.
- Exact CI `34667414544` on `f28d9ac2...` passed verify and hosted negative rootless/AppArmor. Production coverage reached lines `2072/2072` and functions `202/202`; branch coverage reached `460/460`. The canonical source-region gate retained one uncovered region at the `plan_at` propagation of `runtime_identity()` failure, so 100% coverage was not claimed.
- `c41f721e3044c44fca7e1e1350bb48c935429436` introduced a private identity-source seam that preserves public `plan_at` behavior and validation/expiry ordering while allowing the existing typed `RuntimeIdentityUnavailable` path to be exercised through launch planning. The purpose-bound source-fix workflow removed itself in the same ordinary descendant after full tests, Clippy `-D warnings`, rustdoc `-D warnings`, formatting, and `git diff --check` passed.
- `1f0b2d8607407e4057fc9fdfffde72364ff1a80e` reached the repository's canonical production coverage and branch-coverage admissions. Its uploaded production evidence reported lines `2120/2120`, functions `205/205`, and branches `460/460`; raw LLVM regions remained `2813/2816`, so that raw counter is not represented as 100%.
- Review of that exact lineage found an inconsistent proof grammar: `parse_backend_identifier()` admitted upper-case ASCII hexadecimal while the create-receipt parser admitted only lower-case hexadecimal. `db42f814265dbf99c807b843000dc8aa93cc8ba7` added a causal regression requiring `A` repeated 64 times to fail before application-service lifecycle use. Native CI run `34668364957` reached the Test step after repository validation and formatting, then failed as expected on the new regression; the hosted negative rootless/AppArmor lane remained green. This is the RED evidence for the grammar inconsistency.
- The selected causal repair is deliberately smaller than receipt/lifecycle behavior: replace broad ASCII-hex admission in the stdout parser with the same digit-or-`a`-through-`f` predicate used by the receipt boundary. It does not lowercase untrusted input and does not change public error taxonomy, cleanup precedence, retry policy, lifecycle ordering, or resource planning. Exact GREEN SHA and native whole-head evidence remain pending until the source-fix descendant is published and revalidated.

## Evidence hierarchy and remaining gates

The fake-Podman suite proves authority selection, error precedence, cleanup targeting, parser/ACL behavior, and lifecycle call ordering. It does not prove rootless Podman, cgroup, tmpfs, seccomp, AppArmor/SELinux, or network enforcement on a live sandbox.

Issue #113 remains open until the exact current candidate is independently reviewed, inherits the stabilized canonical foundation through ordinary non-force integration, obtains the dedicated positive effective-LSM/runtime evidence, and reaches a protected integrated head. No version, tag, package, GitHub Release, consumer pin, or release claim is authorized by this note.
