# Application-service lease ownership gap-owner repair

## Scope

PR #6 owns caller-scoped application-service lease ownership and idempotency in the Supporting `application_service` bounded context. `LeaseOwnerId`, `ApplicationServiceBackend`, `ApplicationServiceCoordinator`, active-lease ownership, idempotent replay, bounded expiry cleanup, and cleanup fairness remain #6 domain truth. Backend invocation identity (#20/#40), consumer authentication, durable restart/orphan recovery, and Core sandbox-isolation semantics remain separate owner responsibilities.

`docs/product-technical-gap-baseline.md` is repository-wide Gap authority and is maintained by canonical PR #121. A focused application-service branch must not retain its own live copy of that ledger.

## Retained causal evidence

Historical exact #6 head `8daab565bba1b61e605488c080eb28de371990a3` executed native CI `33962779845`. Verify `101297469596` passed the full test/lint/doc path and hosted negative rootless/AppArmor `101297469544` passed. Coverage remained an actual admission failure: `101297469496` failed during generation and branch coverage `101297469509` reported lines `2356/2442` (96.48%), functions `218/228` (95.61%), regions `3085/3211` (96.08%), and branches `429/492` (87.20%). The report contained both child-owned coordinator gaps and inherited runtime gaps, so it is historical exact-head evidence rather than current GREEN authority.

The #6 lineage also removed an unsupported generic `Command::spawn` retry after no focused `io::ErrorKind` evidence justified it. Root issue #24 native-CI branch-trigger evidence is inherited foundation context; it is not application-service lease-ownership domain truth.

## Single-writer finding and decision

Review `5230212241` found that #6 still changed the repository-wide Gap ledger against exact PR base `0f765af1a4eea83029febee3b24c55cd7e7ce4e1`. The stale delta mixed valid #6 ownership/idempotency history with inherited root state, so deleting it without migration would lose causal context.

The repair was migration-first:

1. preserve the #6-specific ownership/idempotency, coverage, and inherited-prerequisite distinctions in this owner-local record;
2. restore only `docs/product-technical-gap-baseline.md` byte-for-byte to the then-exact base blob `5f17a748cf92810963ea67b30ce54675a7c6d919`;
3. keep repository-wide live Gap updates in #121 rather than copying the latest #121 file into this leaf.

No production Rust, public contract, fixture, or test semantics were changed by that ownership repair. Historical CI does not transfer to moved heads.

## Live-root adoption repair

Fresh review `5230412333` found that live root `feat/runtime-foundation-tdd@5c6a44bb2b35eb17d0315d72db242f4488c3c426` had advanced 33 commits beyond #6's recorded base and that a parent-tree preference would silently discard valid deltas. The independent overlaps were `.github/workflows/ci.yml`, `src/application_service/mod.rs`, and `tests/runtime_boundary_regressions.rs`.

The repair was staged and then completed without force/rebase. Commit `a08a786e7744f6697d93da6f41a1a04796cad10e` first adopted exact root CI blob `d172e830706afc290696c818730e1cf570df2be6`, including `persist-credentials: false` on every checkout. The final ordinary two-parent commit `64283e08d99b353430bc1ce97f20019d89f8fbd0` uses the live root tree as the merge-tree foundation so every root-only coverage/runtime delta survives, while overlaying the #6-owned coordinator, backend, package and focused-test deltas.

The two semantic overlaps were merged explicitly rather than hidden by an evil merge. `src/application_service/mod.rs` blob `74270f29a1fe60b6e3a739514a5f17ab685bb3b4` keeps the #6 coordinator module/export and the root parser/coverage simplification (`split_once`, descendant `skip(1)`, parser-dominated no-colon success). `tests/runtime_boundary_regressions.rs` blob `78b11bf8aebcd08dda86b9137963e16e2fc2e0e8` keeps #6's isolated `tempfile` fixtures while binding fake runtime identity to the root-safe 64-character lower-hex container identifier. Root-only `docs/product-technical-gap-baseline.md`, coverage scripts/tests and Podman/root coverage deltas remain inherited from `5c6a44...`; they are not re-authored as #6-owned changes.

This makes `5c6a44bb2b35eb17d0315d72db242f4488c3c426` an actual ancestor of #6. The resulting exact head must reacquire repository, formatting, full-test, Clippy/rustdoc, complete coverage, review/security, applicable positive-isolation, protected-integration and immutable-release evidence. Predecessor GREEN never transfers. Descendants must adopt this moved parent normally and preserve any overlapping effective-isolation/runtime deltas before they are considered current.
