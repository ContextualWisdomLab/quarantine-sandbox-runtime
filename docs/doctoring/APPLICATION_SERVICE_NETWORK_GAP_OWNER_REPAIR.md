# Application-service network Gap owner repair

Status: migration-first single-writer repair for Draft #23. Review `5229629308` found that this RED-only network owner still carried a repository-wide `docs/product-technical-gap-baseline.md` delta after live Gap authority moved to Draft #121. Draft #23 is currently conflicted relative to its base; this repair removes only the stale global-ledger ownership and does not claim to resolve that dependency conflict.

## Ownership decision

The `application_service` network lane owns its network-identity, effective-attachment and cleanup-authority REDs plus their local TRACEABILITY. Draft #121 owns the repository-wide live product/technical Gap ledger. The safe repair is therefore:

1. retain the network-specific evidence and dependency notes locally;
2. restore `docs/product-technical-gap-baseline.md` byte-for-byte to Draft #23's exact base `0f765af1a4eea83029febee3b24c55cd7e7ce4e1`, whose Gap blob is `5f17a748cf92810963ea67b30ce54675a7c6d919`;
3. leave the branch's existing merge conflict visible for later ordinary non-force dependency repair.

Restoring the base blob is an ownership repair, not a claim that the base ledger is current. #121 remains the live repository-wide authority.

## Network owner evidence retained before restore

### Effective attachment — #22/#23

The staged hostile cases require fail-closed `sandbox_network_binding` when the container's applied network mode is wrong, when the expected attachment is absent, or when an unexpected additional Podman network is attached. These tests are intentionally distinct from real negative-egress proof: backend inspection can prove an attachment contradiction but does not prove packet-level isolation.

### Foreign-safe network cleanup — #41

Test-bearing commit `e8bee063dfcbd8d77e04b00b6cd23dc5c11fa5a5` models a foreign container attached to the invocation network after sandbox container-create failure. Current production's `podman network rm --force` can recursively remove containers using that network. The staged RED instead requires the foreign member to survive, forbids network-level force removal, and expects an in-use non-force removal to surface `ApplicationServiceError::CleanupFailed`. Detailed authority and alternatives remain in `APPLICATION_SERVICE_NETWORK_CLEANUP_OWNERSHIP_TRACEABILITY.md`.

### Acquired network identity — #48

Test-bearing commit `3d7f81680d2ad6659604916c9885567418e3a8a2` models a generated `qsr-net-*` name being rebound to a foreign network after creation. Podman network creation reports the newly created name, while network inspection exposes the full network ID. The staged RED therefore requires identity inspection/acquisition before container creation, ID-bound `--network` selection, preservation of generated names as correlation metadata, and no foreign-network side effect. Detailed reasoning remains in `APPLICATION_SERVICE_NETWORK_IDENTITY_TRACEABILITY.md`.

The eventual GREEN must compose all three controls: acquired network identity, exact/exclusive deny-by-default attachment, and foreign-safe non-force cleanup. A larger random name, a label, or a public lease field is not a substitute for runtime-acquired backend identity.

## Historical execution limitation

Prior exact `4e47d11cc458e96b68ba43ae661871b08aafa158`, native CI `33984794963`, did not establish the intended network REDs. Verify job `101356231373` stopped at `cargo fmt --check` before the Test step. Ordinary commits `603d2642510dad6ac4576298694aad8a3202c738` and `95f9e2c73b03a8b6a38f8accbb363adfea56b723` apply only the reported rustfmt changes to the network-cleanup and network-identity RED fixtures. The current lineage must therefore reacquire execution and show each hostile test failing for its intended semantic reason before any production GREEN is authorized.

This limitation is retained explicitly so a broad historical CI failure cannot be promoted to causal RED evidence after the global ledger is removed from this leaf.

## Downstream prerequisite — #118

The network lane is a direct prerequisite for #118. A one-shot candidate that places `RuntimeLeaseMetadata.network_id`—currently generated `plan.network_name()` correlation data—into private cleanup authority and then executes `podman network rm --force` would still grant destructive authority to a re-resolvable name. Private visibility does not make a mutable correlation selector immutable. #118 must remain Draft until canonical network ownership is repaired and adopted through ordinary ancestry.

## Rejected alternatives

- Copy #121's latest global ledger into #23: rejected because it preserves multiple writers and makes integration order-dependent.
- Restore the base ledger without preserving network evidence first: rejected because the leaf ledger carries staged #41/#48 ownership rationale, the non-causal CI limitation, and the #118 prerequisite relationship.
- Treat the branch conflict as permission to force-rebase: rejected; conflict/dependency repair remains a separate non-force adoption task.
- Claim the rustfmt-clean current head as executed RED: rejected because predecessor execution stopped before tests.
- Add production network GREEN now: rejected until the exact REDs execute for the intended causes.

## Remaining gates

This owner repair changes no production runtime behavior, public schema, test semantics, network selector, cleanup command, or release claim. Draft #23 remains open and conflicted until its prerequisites are adopted non-force. After dependency repair, the current exact network REDs must execute independently; only then may the minimum production repair bind lifecycle operations to acquired network identity, reject attachment ambiguity, remove network-level force deletion, and retain cleanup-failure visibility. Real rootless name-rebind/foreign-member/negative-egress E2E, positive effective-LSM evidence, full owned coverage/rustdoc, review/security, protected integration, and immutable release evidence remain independent completion gates.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
