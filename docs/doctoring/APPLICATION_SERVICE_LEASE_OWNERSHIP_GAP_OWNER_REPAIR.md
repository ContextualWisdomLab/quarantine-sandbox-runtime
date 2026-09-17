# Application-service lease ownership gap-owner repair

## Scope

PR #6 owns caller-scoped application-service lease ownership and idempotency in the Supporting `application_service` bounded context. `LeaseOwnerId`, `ApplicationServiceBackend`, `ApplicationServiceCoordinator`, active-lease ownership, idempotent replay, bounded expiry cleanup, and cleanup fairness remain #6 domain truth. Backend invocation identity (#20/#40), consumer authentication, durable restart/orphan recovery, and Core sandbox-isolation semantics remain separate owner responsibilities.

`docs/product-technical-gap-baseline.md` is repository-wide Gap authority and is maintained by canonical PR #121. A focused application-service branch must not retain its own live copy of that ledger.

## Retained causal evidence

Historical exact #6 head `8daab565bba1b61e605488c080eb28de371990a3` executed native CI `33962779845`. Verify `101297469596` passed the full test/lint/doc path and hosted negative rootless/AppArmor `101297469544` passed. Coverage remained an actual admission failure: `101297469496` failed during generation and branch coverage `101297469509` reported lines `2356/2442` (96.48%), functions `218/228` (95.61%), regions `3085/3211` (96.08%), and branches `429/492` (87.20%). The report contained both child-owned coordinator gaps and inherited runtime gaps, so it is historical exact-head evidence rather than current GREEN authority.

The #6 lineage also removed an unsupported generic `Command::spawn` retry after no focused `io::ErrorKind` evidence justified it. Root issue #24 native-CI branch-trigger evidence is inherited foundation context; it is not application-service lease-ownership domain truth.

## Single-writer finding and decision

Review `5230212241` found that #6 still changed the repository-wide Gap ledger against exact PR base `0f765af1a4eea83029febee3b24c55cd7e7ce4e1`. The stale delta mixed valid #6 ownership/idempotency history with inherited root state, so deleting it without migration would lose causal context.

The repair is therefore migration-first:

1. preserve the #6-specific ownership/idempotency, coverage, and inherited-prerequisite distinctions in this owner-local record;
2. restore only `docs/product-technical-gap-baseline.md` byte-for-byte to exact-base blob `5f17a748cf92810963ea67b30ce54675a7c6d919`;
3. keep repository-wide live Gap updates in #121 rather than copying the latest #121 file into this leaf.

No production Rust, public contract, fixture, or test semantics are changed by this ownership repair. No force push or destructive rebase is permitted. Historical CI does not transfer to the moved head; the resulting exact head must reacquire its own repository, formatting, full-test, Clippy/rustdoc, coverage, review/security, applicable positive-isolation, protected-integration, and immutable-release evidence.

## Live-root adoption repair

Fresh review `5230412333` found that the remaining root adoption cannot be represented safely by preferring either parent tree. Live root `feat/runtime-foundation-tdd@5c6a44bb2b35eb17d0315d72db242f4488c3c426` is 33 commits ahead of the PR's recorded base snapshot `0f765af1a4eea83029febee3b24c55cd7e7ce4e1`, and three paths carry independent valid deltas on both sides: `.github/workflows/ci.yml`, `src/application_service/mod.rs`, and `tests/runtime_boundary_regressions.rs`.

The first overlap has an unambiguous successor. Root CI changes the push target to protected `develop` and sets `persist-credentials: false` on every checkout. Child exact `ecdd84836d1d04660f620156f2190d8eb5664837` already carried the `develop` target but not the credential hardening. Commit `a08a786e7744f6697d93da6f41a1a04796cad10e` therefore adopts the root CI blob `d172e830706afc290696c818730e1cf570df2be6` by ordinary fast-forward without changing production or test semantics.

The two semantic overlaps remain intentionally unresolved rather than hidden in an evil merge. Root `application_service/mod.rs` contains the current parser-dominated repository-name simplification required by the exact coverage contract, while #6 adds the coordinator module/export. Root `runtime_boundary_regressions.rs` binds fake container identity to the current safe-identifier grammar, while #6 independently moved process fixtures into isolated `tempfile` directories. The eventual two-parent adoption must preserve both intents explicitly, make `5c6a44bb2b35eb17d0315d72db242f4488c3c426` an actual ancestor, and then reacquire exact-head CI. Predecessor GREEN is not transferable.
