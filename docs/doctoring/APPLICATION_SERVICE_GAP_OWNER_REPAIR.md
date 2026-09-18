# Application-service Gap owner repair

Status: migration-first single-writer repair for Draft #21. Review `5229612380` found that this application-service owner still carried a repository-wide `docs/product-technical-gap-baseline.md` delta after live Gap authority moved to Draft #121. The leaf delta is not deleted until the stable application-service causal history that it uniquely summarized is retained here.

## Ownership decision

`application_service` owns its runtime/lifecycle domain truth and local TRACEABILITY. Draft #121 owns the repository-wide live product/technical Gap ledger. A leaf PR must not copy the latest #121 ledger, retain a stale global snapshot, or use a mutable sibling head as authority.

The repair is deliberately two-step and ordinary/non-force:

1. preserve the stable #20/#40/#42/#113 evidence and later hosted candidate measurements in this owner-local document;
2. restore `docs/product-technical-gap-baseline.md` byte-for-byte to Draft #21's exact base `5c6a44bb2b35eb17d0315d72db242f4488c3c426`, whose Gap blob is `bacb346f2ce4259a4f55bd3bece5e871b06d69db`.

The checked-in repository-wide current authority remains #121. Restoring the base blob here is an ownership repair, not a claim that the base ledger is current.

## Stable causal evidence retained before restore

### Runtime invocation identity — #20

Exact `368e20eeeb0ac3d573a913af981dcb5dd4104b1a`, native CI `34301336763`, verify `102308701305`, causally proved that two independent same-request/same-second launches could both select `qsr-app-2500e69e869f94b7`. Candidate `850ace94f578e53329a8b9f2465542f3224f8f48` introduced typed entropy failure and `896e25db4753193cc43a7ff319a47efb0cfe0da5` introduced the 128-bit operating-system-entropy invocation identity. Consumer `request_id` remains correlation/idempotency data rather than destructive authority. Detailed reasoning remains in `APPLICATION_SERVICE_RUNTIME_IDENTITY_TRACEABILITY.md`.

### Serialized lease versus destructive authority — #42

Exact `1d0cf2f47a8bd9df6594c734806a6c9c912fe0ed`, native CI `34299565806`, verify `102303404021`, causally proved that a caller-deserialized lease could select forged `foreign-container` / `foreign-network` identifiers for destructive cleanup. Candidate `b0bcbe90ef034115ec266ffed5937aed1ee75150` captures crate-private, non-serializable cleanup authority during runtime construction and `e15980b820becd13c7e5756a3adca1f6504cd92c` requires that authority before Podman destruction. Detailed reasoning remains in `APPLICATION_SERVICE_CLEANUP_AUTHORITY_TRACEABILITY.md`.

### Exact acquired container lifecycle authority — #40

The #40 causal lineage proved that generated `qsr-app-*` correlation names could remain lifecycle/destructive selectors after Podman had already returned an exact container ID. Current #21 ancestry retains the repair that threads the admitted long ID through start, inspect/top, port, failure cleanup, readiness cleanup, termination, and private cleanup authority. Generated names remain public correlation/audit metadata and are never a post-create destructive fallback. Detailed causal and source lineage remains in `APPLICATION_SERVICE_LIFECYCLE_OWNERSHIP_TRACEABILITY.md`.

### Successful-create receipt recovery and identifier grammar — #113

Issue #113 adds an independent runtime-owned `--cidfile` receipt so a successful create with malformed stdout can recover only an exact created-container identity. Generated correlation names are never substituted as cleanup authority. A later exact-head RED `db42f814265dbf99c807b843000dc8aa93cc8ba7`, native CI `34668364957`, proved that stdout admission still accepted a 64-character upper-case hexadecimal identifier while receipt admission required lower-case. Minimum production `31cc048a8a94e73dca49ee5d2c097582859b039a` aligned both proof channels to exactly 64 ASCII lower-case hexadecimal bytes without normalization. Detailed receipt and failure-path evidence remains in `APPLICATION_SERVICE_CREATE_IDENTIFIER_TRACEABILITY.md` and `APPLICATION_SERVICE_CREATE_RECEIPT_TRACEABILITY.md`.

## Consolidated hosted candidate evidence

Exact descendant `b3de103f4dd1fe86f5fac2785e3fa58a45f18674`, native CI `34669310300`, reacquired repository policy, formatting, full workspace/all-target tests, Clippy, rustdoc, production coverage, branch coverage, and hosted negative rootless/AppArmor GREEN after the above semantic repairs and traceability correction.

Its admitted coverage evidence was:

- physical production lines `2123/2123`;
- functions `205/205`;
- canonical source regions `2819/2819`;
- branches `462/462`;
- raw LLVM regions remained diagnostic `2816/2819` rather than replacing canonical source-region admission;
- production coverage artifact `10290410900`, SHA-256 `ca70d1fb157ea0e79b5e6537917d33fb116871e10b02fd42300b16cd4339a6b8`;
- branch artifact `10290586575`, SHA-256 `b2610c782c8e6d7153be3b18aacc0c930d8a22891e29f7c8bd721b8b51aa6069`.

The same candidate executed the create-receipt edge matrix, malformed-successful-create exact-ID cleanup, no-trustworthy-receipt fail-closed control, exact-ID post-create lifecycle, forged-lease authority rejection, invocation-identity separation, cleanup precedence, and receipt OS-edge tests. Dedicated positive effective-LSM job `103487502163` remained queued. This is historical exact-head candidate evidence only; it does not transfer to any moved head.

Draft #21 later advanced through test hardening and documentation to exact `65f69de6eb1cf78b316b38424f8c35c316cd0672`. Review `5229612380` is therefore followed by this owner-local migration and a separate exact-base Gap restore. The moved head must reacquire its own CI/review/security/isolation gates.

## Rejected alternatives

- Copy #121's latest global ledger into #21: rejected because it creates another live writer and makes later integration order-dependent.
- Restore the old base ledger without first retaining owner-specific evidence: rejected because the leaf Gap delta contains stable causal and hosted evidence that is not merely a pointer update.
- Force-rebase or rewrite the branch to drop the file: rejected because ordinary ancestry preserves intervening deltas and review evidence.
- Close #21 because newer application-service/network successors exist: rejected until every valid #21 delta, test, fixture, contract, and evidence item is demonstrably succeeded or normally integrated.

## Remaining gates

The ownership repair changes no production Rust, schema, test semantics, cleanup authority, identity grammar, network behavior, or release claim. Draft #21 remains open until one unchanged dependency-safe exact head satisfies repository validation, rustfmt, full locked workspace/all-target tests, Clippy, public/private rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, applicable real rootless and positive effective-LSM evidence, protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication.

## References

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Podman Authors. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Davis, K. R., Peabody, B. G., & Leach, P. J. (2024). *Universally unique IDentifiers (UUIDs)* (RFC 9562). RFC Editor. https://doi.org/10.17487/RFC9562
