# Application-service network owner adoption traceability

Status: current owner-ancestry and exact-head evidence repair for Draft #127.

## Finding

Draft #127 had remained on application-service owner #21 exact `65f69de6eb1cf78b316b38424f8c35c316cd0672` while the live #21 branch had advanced to `717edef989d8b6bb7e1f672d91d9258f7942a6b2`. Fresh compare resolved the old #21 exact as the merge base: #127 exact `4306462ddbd90ce137074e70af7c9c0bbee18d9e` was 36 commits ahead and two commits behind the live owner.

The two missing #21 commits were ownership/documentation repairs only:

- `6a6bb6a27390504220c8b3f6aef7f280e4318c2e` migrated the stable #20/#40/#42/#113 causal and hosted evidence into `APPLICATION_SERVICE_GAP_OWNER_REPAIR.md`;
- `717edef989d8b6bb7e1f672d91d9258f7942a6b2` restored `docs/product-technical-gap-baseline.md` to #21's exact-base blob so Draft #121 remains the repository-wide single writer.

Neither commit changes production Rust, schema, tests, identifier grammar, cleanup authority, network selection, nor application-service behavior. Nevertheless, leaving #127 outside the current canonical owner ancestry would make the ownership boundary and later successor proof stale.

## Repair

Two-parent merge `7934aa99085264db7ee654088c3df0f116a67b66` has parents:

1. #127 predecessor `4306462ddbd90ce137074e70af7c9c0bbee18d9e`;
2. live #21 `717edef989d8b6bb7e1f672d91d9258f7942a6b2`.

The #127 ref was advanced with `force=false`. The merged tree preserves every #127 network-lifecycle delta and adopts #21's owner-local evidence plus global-Gap single-writer restore. No source copy, destructive rebase, or predecessor-status promotion was used.

## Exact owner evidence

#21 native CI `35165254755` is fully classified on exact `717edef...`:

- verify `105024795504`: GREEN through exact checkout, dependency lock, repository policy, coverage parser, rustfmt, full workspace/all-target tests, Clippy, and rustdoc;
- production coverage `105024795320`: GREEN;
- branch coverage `105024795584`: GREEN;
- hosted negative rootless/AppArmor `105024795469`: GREEN;
- positive SELinux `105024795428`: no eligible runner, cancelled after 24 hours without executing steps.

The workflow is therefore not release-GREEN. These exact parent results prove the adopted owner delta is repository-fit on hosted lanes, but they do not transfer to #127 after ancestry movement.

## Current #127 exact-head fixture RCA

Exact `d2c22a423870e5566b198587e285ecbf66ea0526` eventually acquired hosted runners in native CI `35460751019`; it is no longer valid to classify that run as merely queued.

- verify `105944054999` passed exact checkout, dependency lock, repository policy, coverage-parser tests, and `cargo fmt --check`, then failed during the full locked workspace/all-target test suite;
- the first failing test was `podman_application_service_identity_race_red::independent_same_request_launches_use_distinct_runtime_owned_resource_identities`, which aborted with `MalformedIsolationInspection { operation: "network_identity_inspect" }` before either launch could exercise the intended independent-resource-identity assertions;
- production coverage `105944054990` and branch coverage `105944054951` acquired hosted runners and failed while executing the same test corpus, so those failures do not establish a coverage deficit independently of the fixture prerequisite;
- hosted rootless/AppArmor negative job `105944054969` completed GREEN;
- positive SELinux job `105944054864` never acquired its required self-hosted runner (`runner_id=0`, no steps) and was cancelled after 24 hours. That remains runner-capacity evidence, not a product-code result.

RCA is test-local. The identity-race fake Podman still returned only `internal`/`dns_enabled` fields for `network inspect`, while current production acquires and validates a backend network ID before container creation. Test-only commit `4053fc1a245a2a49b619f4d26db15cd07bc98087` repairs that prerequisite without changing production Rust/API/schema/network behavior. For each exact generated `qsr-net-*` correlation name the fixture derives a distinct canonical 64-lower-hex backend ID from the invocation identity and returns it together with `internal=true` and `dns_enabled=false`. This avoids the weaker false-GREEN of assigning both concurrent launches one static backend network ID.

The repair does not claim the network-owner contract is GREEN. The production path still has active owner REDs around exact acquired network authority, cleanup, effective attachment, and the #128 hold/attest/release boundary. The moved exact must execute independently; predecessor results do not transfer.

#127 must independently execute its own current exact before any network semantic or merge claim. The active network sequence remains provenance → no cleanup before exact-ID admission → exact-ID non-force cleanup → exact effective attachment → private destructive authority → foreign-safe termination → mandatory effective isolation evidence → #128 hold/attest/release.

## DDD boundary

`application_service` remains the Supporting bounded context that owns runtime identity and lifecycle authority. The network-lifecycle successor may extend that owner only through ordinary ancestry. Generated `qsr-*` names remain correlation data; destructive authority must be runtime-acquired, exact, private, and non-serializable. Draft #121 alone owns the repository-wide live Gap ledger.

## Gate

This document records ancestry and evidence only. It does not make #127 release-ready and does not authorize child closure. Current #127 and every descendant must reacquire exact-head repository validation, tests, Clippy/rustdoc, complete owned-production coverage, qualifying review/security/thread gates, applicable real runtime and positive effective-LSM evidence, protected integration, and immutable publication evidence.