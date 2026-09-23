# Application-service successful-lease network authority traceability

## Decision status

Proposed RED above canonical network owner #127 exact `b81468346a88c37ed920f6417571b9e7cc846b4d`. This slice changes tests and TRACEABILITY only. Production Rust, public wire shape, schemas, network creation, and cleanup implementation remain unchanged.

## Problem

Current launch already acquires one creation-bound Podman network ID, proves P0 state by that exact ID, binds container creation to the exact ID, and carries the ID through post-start effective-isolation checks. The authority is then lost at successful lease publication.

`RootlessPodmanAdapter::launch_at()` constructs `RuntimeLeaseMetadata.network_id` from `plan.network_name()`, the generated `qsr-net-*` public correlation value, while `terminate_at()` consumes `ApplicationServiceCleanupAuthority.network_id()` as a destructive selector and invokes `podman network rm --force <selector>`.

The resulting lifecycle has an authority downgrade after successful publication: an exact admitted backend identity controls launch and verification, but explicit termination can fall back to re-resolvable public correlation metadata and network-level force deletion. A same-name replacement or foreign network member can therefore turn consumer-visible correlation into destructive authority.

## Required invariant

Successful launch must preserve two different network identities with different roles:

- public correlation metadata may remain the generated `qsr-net-*` value for compatibility and audit;
- private lifecycle authority must retain the exact admitted 64-lower-hex backend network ID that was used for container binding and P0 verification.

Explicit termination may remove only the private exact admitted ID and must use non-force network removal. If Podman refuses removal because a foreign member exists, cleanup fails closed rather than deleting the foreign resource. Public correlation, labels, prefixes, event membership, or re-resolution are not destructive authority.

This is downstream of pre-admission authority #144: #144 prevents a merely nominated candidate from becoming destructive authority before P0 admission; this slice prevents already-admitted exact authority from being discarded when a successful lease is published.

## Executable RED

`tests/podman_application_service_successful_lease_network_authority_red.rs` uses current #127 launch mechanics end to end:

1. rootless/security prerequisites succeed;
2. `network create` records the generated correlation;
3. bounded creation history emits one uppercase-`Network` receipt intentionally, so this lane does not pre-apply the separate Podman-v6 lowercase-`network` parser repair;
4. exact-ID P0 inspection admits `OWNED_NETWORK_ID`;
5. container creation must bind `--network OWNED_NETWORK_ID`;
6. effective attachment and post-start network inspection stay bound to the same exact ID;
7. the published lease continues exposing generated correlation metadata;
8. explicit termination must execute exactly `network rm OWNED_NETWORK_ID` and must execute no `network rm --force ...` or public-correlation removal.

Current production is expected to fail the final lifecycle assertions because successful-lease cleanup authority is derived from the public network correlation and explicit termination adds `--force`.

## Minimum GREEN direction

Keep public serialized lease evidence and private cleanup capability separate. The smallest owner-local repair is to retain the admitted network ID in the crate-private/non-serializable cleanup authority at lease construction while leaving the public correlation field unchanged, then make explicit termination call the existing exact non-force admitted-network cleanup path or an equivalent single implementation.

Do not serialize private destructive selectors merely to make termination work. Do not derive private authority from deserialized lease evidence. Do not change public `network_id` semantics without an explicit versioned contract migration. Do not broaden this repair into #141 durable recovery, #144 pre-admission authority, event-key casing, remote clock-domain parity, applied-image evidence, or lease-deserialization admission.

## DDD boundary

`application_service` owns lease lifecycle coordination and the split between public evidence and private cleanup capability. `infrastructure::podman` owns the concrete Podman selector and removal operation. Public `ApplicationServiceLease` remains consumer evidence; crate-private `ApplicationServiceCleanupAuthority` remains the capability that authorizes destructive lifecycle operations. The exact backend network ID belongs only in the latter unless a separately versioned public contract says otherwise.

## Evidence and references

Current source authority is `src/infrastructure/podman.rs` on #127 exact `b81468346a88c37ed920f6417571b9e7cc846b4d`: launch passes the admitted `network_id` through container binding and `verify_effective_isolation()`, then constructs `RuntimeLeaseMetadata.network_id` from `plan.network_name()`; `terminate_at()` uses cleanup-authority `network_id()` with `podman network rm --force`.

Historical #120 already established the foreign-member explicit-termination requirement on an older network-owner shape. This current-shape witness does not close #120; it is a succession candidate that must preserve #120's foreign-member evidence after dependency-safe owner repair.

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-rm.1.html

## Completion gate

Keep this RED Draft until one unchanged exact executes the successful-launch path and fails for the intended private-authority/non-force termination cause. Then apply only the minimum owner repair, re-run this witness together with #120 foreign-member semantics and existing exact-ID binding/P0/partial-cleanup tests, and reacquire repository policy, rustfmt, full locked workspace/all-target tests, Clippy/rustdoc, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, positive effective isolation, dependency-safe parent succession, protected integration, and immutable release evidence.
