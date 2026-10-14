# Podman missing-isolation-evidence Gap owner repair

Reviewed 2026-09-17 KST against Draft PR #89 exact `4e05824864489d24ee786ee4ffc40c3c96842de6`, exact base `78281e244c530dcafb3368b9f1d9896e846206a9`, and repository-wide Gap owner PR #121.

## Owner-boundary finding

Issue #73 is a focused `infrastructure::podman` anti-corruption-layer repair. It owns the distinction between an explicitly observed secure Podman value and absence of the corresponding inspection field; it does not own the repository-wide live product/technical Gap ledger. Review `5229912324` found that the current leaf still changes `docs/product-technical-gap-baseline.md`. Carrying that historical ledger forward could replay stale root, worker, release, and owner pointers over #121 during later integration.

The repair is migration-first. Stable issue #73 evidence remains in `PODMAN_MISSING_ISOLATION_EVIDENCE_TRACEABILITY.md` and is summarized here before restoring the global Gap file byte-for-byte to this PR's exact-base blob `1926d27d6ea98edbbddfee2c29993d2aa549c1f6`.

## Retained causal evidence

Test-bearing `aec598db7579b09854e15b2d22ad127dccbe56cc` introduced three hostile missing-field cases plus an explicit-secure-value control. Exact `cac5f37e03b30c8b5f73911e7388aab34e23c50c`, native CI `34132414062`, coverage job `101775434743`, executed the intended RED: missing `EffectiveCaps` and `BoundingCaps` fell through to `ReadinessTimeout` instead of malformed `container_inspect`; missing `dns_enabled` fell through to `ReadinessTimeout` instead of malformed `network_inspect`; explicit empty capability arrays and explicit `dns_enabled=false` remained acceptable observations. Verify on the same exact source exposed only a formatting prerequisite for this lane.

The same predecessor's hosted negative rootless/AppArmor job `101775434654` stopped before product E2E when the immutable fixture pre-pull received Docker Hub HTTP 500. Leak cleanup succeeded. That event is external runtime/CI availability evidence, not product isolation failure and not positive confinement authority.

Production `80e6fd19ab5ef11eab3d18b3e80b43f880ce6058` is the minimum semantic repair: remove only the secure-default fallbacks from `ContainerInspection.EffectiveCaps`, `ContainerInspection.BoundingCaps`, and `NetworkInspection.dns_enabled`. Missing required members then fail through the existing malformed-inspection ACL, while explicit `[]`, `[]`, and `false` retain their meaning. Parser taxonomy, process capabilities, seccomp/LSM, network-internal behavior, readiness, cleanup, and application-service domain semantics are unchanged.

The branch later adopted root exact `78281e244c530dcafb3368b9f1d9896e846206a9` by ordinary two-parent commit `4e05824864489d24ee786ee4ffc40c3c96842de6`, preserving the issue #73 tree while making the root an actual ancestor. Earlier CI on `80e6fd...` was cancelled after branch movement and does not transfer.

## DDD and decision

`infrastructure::podman` owns backend JSON translation and malformed-evidence classification. `sandbox_execution` owns backend-neutral isolation truth. `application_service` consumes the stable failure/evidence vocabulary. Absence at a P0 security evidence boundary is unavailable evidence, never an inferred secure value.

Selected repair: retain focused production/test/TRACEABILITY, add this local owner-repair record, then restore only the repository-wide Gap file to the exact base. Rejected alternatives are copying #121's newest ledger into this leaf, deleting causal history, treating the leaf as a second live Gap writer, or force-rebasing merely to remove the file.

After the docs-only head movement, predecessor checks do not transfer. The moved head must independently reacquire repository validation, formatting, locked full tests, Clippy/rustdoc, complete owned-production and branch/edge coverage, qualifying review/security gates, real rootless and positive effective-LSM evidence, protected integration, and immutable release evidence.
