# Application-service create-identifier authority traceability

Status: Issue #113 has an executed causal RED and a minimum exact-ID receipt repair on Draft #21 ancestry. Source-fix run `34660711504` validated the repair broadly before publishing ordinary descendant `3a37a4f7073a6d159844ec0f37bb5bc1df0772ea`. That bot-authored descendant received an `action_required` native CI run with zero jobs, so the same source must still reacquire ordinary exact-head PR CI before merge or release authority is claimed.

## Problem and security boundary

`RootlessPodmanAdapter::launch_at` receives two distinct identities around successful `podman create`:

- the generated `qsr-app-*` name, which is correlation/audit metadata chosen before creation;
- the backend-acquired container ID, which is the only acceptable post-create lifecycle and destructive container authority.

The earlier create-ID repair correctly rejected name-like, short and non-hex stdout before promoting it to lifecycle authority. A separate edge remained: `podman create` can return success while stdout is malformed. Earlier production then called generated-name cleanup. That was not safe: a requested name is a re-resolvable selector and does not prove which resource was created. Removing that cleanup without another identity source would avoid deleting the wrong container but could leave a successfully created container orphaned.

Issue #113 therefore requires an independent runtime-owned create receipt. The current repair passes a private `--cidfile` to Podman and admits only one exact 64-character lowercase hexadecimal receipt value for recovery. The generated name never becomes fallback destructive authority.

## Authoritative backend contract

Podman's current `podman-create(1)` documentation states that a successful create prints the container ID to stdout and that `--cidfile=file` writes the container ID to a file. These are two backend-produced channels for the identity of the created resource. The runtime uses the receipt as independent recovery evidence when stdout cannot be admitted; it does not infer identity from a mutable name.

The security rule is intentionally asymmetric:

1. valid stdout plus an absent receipt can continue with the already admitted exact stdout ID;
2. valid stdout plus a matching receipt uses the same exact ID;
3. valid stdout plus a different valid receipt fails closed after cleaning only the receipt-owned exact ID;
4. malformed stdout plus a valid receipt cleans only that exact receipt ID and preserves the original `MalformedIsolationInspection { operation: "container_create" }`;
5. malformed stdout plus no trustworthy receipt cleans only the invocation-owned network and returns `MalformedIsolationInspection { operation: "container_create_receipt" }`;
6. no path is allowed to convert the generated `qsr-app-*` name into destructive container authority.

The missing-receipt case deliberately remains an explicit unreconciled-identity condition. There is no safe container deletion target to invent. Crash/restart orphan reconciliation is a separate Recovery-context gap and must not be approximated with name lookup.

## Causal RED

Issue #113 was staged on canonical application-service Draft #21 rather than command-runtime #112, preserving the single-writer bounded-context boundary.

The exact source-fix execution reproduced two independent causal failures before applying production changes:

- `podman_application_service_malformed_create_receipt_red::malformed_successful_service_create_uses_runtime_receipt_for_exact_id_cleanup` expected the original malformed-create error after exact receipt-ID cleanup but observed `Err(CleanupFailed)` because production still attempted generated-name removal;
- `podman_application_service_missing_create_receipt_red::malformed_successful_create_without_receipt_never_uses_generated_name_for_cleanup` expected explicit `container_create_receipt` failure without destructive container action but likewise observed `Err(CleanupFailed)` from generated-name cleanup.

Both failures were reproduced in source-fix run `34660711504` before the repair step. They reached the intended ownership boundary rather than failing on checkout, dependency, formatting or repository-policy prerequisites.

An earlier repair attempt also exposed a stale `root_coverage_edges` fake-backend mode. After the production repair was applied, that fixture advanced to `ReadinessTimeout` because its shell failure token still used the old `malformed_identifier_cleanup_failure` name. This was fixture drift, not a production semantic failure. Commit `dafb0722446b1cb727c8018c06f657c629bde312` aligned the two stale shell tokens while retaining generated-name `rm` failure as a negative-control tripwire.

## Minimum GREEN

Source-fix run `34660711504` then applied the minimum repair and proved focused GREEN for:

- malformed successful create with a valid runtime-owned receipt;
- malformed successful create with no trustworthy receipt;
- the `root_coverage_edges` negative-control family.

The same run subsequently passed repository validation, full locked workspace/all-target tests, Clippy with `-D warnings`, rustdoc with `-D warnings`, rustfmt and `git diff --check`. It removed its temporary source-fix workflow and repair scripts before publishing ordinary descendant `3a37a4f7073a6d159844ec0f37bb5bc1df0772ea` (`fix(application-service): recover exact create identity`).

Production now creates a private temporary receipt directory, inserts `--cidfile=<runtime-owned-path>` into the application-service create argv, reads the receipt through a bounded identity admission helper, and removes the obsolete generated-name cleanup helper. All admitted post-create start/inspect/top/port/stop/remove behavior remains bound to the acquired exact container ID through the existing #40 lifecycle repair. Public `sandbox_id` remains correlation evidence; private cleanup authority retains the exact acquired container ID.

The source-fix GREEN is strong implementation evidence, but it is not a substitute for exact-head PR CI. The first native CI attached to bot-authored `3a37a4f...` concluded `action_required` without materializing jobs. A later user-authored documentation descendant must therefore reacquire the full native CI/coverage/security lanes on the unchanged production repair before this document calls the candidate exact-head GREEN.

## Invariants preserved

- `request_id` remains consumer correlation, not runtime resource identity.
- generated `qsr-app-*` names remain audit/correlation metadata and are never post-create destructive authority.
- successful admitted-ID lifecycle commands remain exact-ID bound under #40.
- public/deserialized lease fields cannot recreate cleanup authority under #42.
- network ownership/foreign-safe cleanup remains separately governed by the network-binding lane.
- receipt disagreement or malformed identity fails closed; there is no name, short-ID or normalization fallback.
- cleanup failure still takes precedence when exact-ID cleanup itself cannot be proven successful.
- no retry, sleep, force-rebase, provider-specific fallback or mutable sibling dependency was introduced.

## Rejected alternatives

- **Delete by generated name after malformed stdout.** Rejected because a name is mutable/re-resolvable and is not backend-acquired ownership evidence.
- **Do nothing after successful create with malformed stdout.** Rejected when an exact receipt is available because it knowingly leaks a recoverable runtime-owned resource.
- **Resolve the generated name with a later inspect/list query.** Rejected because the lookup first selects a potentially foreign resource; a later ID check does not make the initial selector authoritative.
- **Accept short IDs or arbitrary tokens.** Rejected because abbreviation or token normalization widens the destructive selector grammar.
- **Hide missing receipt behind the original stdout error.** Rejected because the security-relevant unreconciled-resource condition must remain explicit evidence.
- **Treat test-only fake backend success as positive confinement evidence.** Rejected. These regressions prove identity/cleanup control flow only; real rootless, positive-LSM, cgroup/mount/network and release evidence remain separate gates.

## Remaining release evidence

Draft #21 stays open. Required work still includes native exact-head CI and complete owned-production coverage on the current descendant, qualifying independent review/security gates, real rootless resource/network/cleanup acceptance, dedicated positive effective-LSM evidence, ordinary protected integration, and immutable version/package/SBOM/provenance/reproducibility/rollback publication. Issue #113 remains open until those release-level requirements are satisfied.

## References

Podman. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (2026). *podman-start — Start one or more containers*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Chandramouli, R. (2017). *Security assurance requirements for Linux application container deployments* (NIST Interagency Report 8176). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.IR.8176
