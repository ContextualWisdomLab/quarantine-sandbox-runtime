# Podman post-admission network identity traceability

## Finding

Draft #127 exact `e9ad58ceaec006b11da63772c22447b3a424da23` already separates the generated `qsr-net-*` correlation from a canonical-format backend network ID used for container creation. That current parent does **not** yet prove that the ID is provenance-admitted immutable authority: `NetworkIdentityInspection` carries only `id`, and `acquire_network_id()` validates only canonical backend-identifier grammar. The same parent separately carries `tests/podman_application_service_network_identity_provenance_owner_red.rs`, whose expected-name / `internal=true` / DNS-disabled admission contract is still RED.

This child therefore models the next boundary **after** creation-bound identity and provenance/P0-state admission are repaired on the canonical parent. Even with those parent obligations satisfied, a later boundary can still lose the admitted authority.

`RootlessPodmanAdapter::launch_at()` obtains `network_id` from `acquire_network_id()` and replaces the container `--network` selector with that value. After the exact acquired container starts, `verify_effective_isolation()` receives the launch plan and container ID, but not the network ID. Its P0 network-state check therefore executes `podman network inspect --format json <plan.network_name()>` again.

That second name resolution is not equivalent to verification of the admitted object. Between identity admission and effective isolation verification, another object can occupy the public correlation while preserving `internal=true` and DNS-disabled state. The later check can then describe the replacement rather than the network selected by immutable ID for the exact container.

This is distinct from both parent #127 identity REDs. The create→first-inspect witness asks whether the first inspected ID belongs to the object created by this invocation. The provenance witness asks whether the admitted object also proves expected generated name and required P0 state before container creation. Fixing both does not by itself prevent a later security check from falling back to mutable correlation.

## Exact code and RED

Parent code authority: `src/infrastructure/podman.rs@e9ad58ceaec006b11da63772c22447b3a424da23`.

The current parent flow is:

1. `acquire_network_id(&plan)` returns an ID that satisfies canonical backend-identifier grammar;
2. current parent production does not yet prove expected-name / `internal=true` / DNS-disabled provenance at that admission boundary; the dedicated provenance owner test remains RED;
3. `bind_network_selector(&mut create_args, &network_id)` binds container creation to the canonical-format ID;
4. the exact acquired container is started;
5. `verify_effective_isolation(&plan, ..., &container_id)` runs without `network_id`;
6. the network P0 check resolves `plan.network_name()` again.

Test-only commit `de78cac3add817e4d2bd24316d8f52676128638f` adds `tests/podman_application_service_network_post_admission_rebind_red.rs` on Draft #142. The fake backend returns `OWNED_NETWORK_ID` for the first name lookup and requires container creation to use it. After container start, the same public name resolves to a different canonical ID with otherwise-valid P0 state, while exact-ID inspection of `OWNED_NETWORK_ID` remains available.

The witness requires post-start P0 network inspection to use the same immutable network ID selected for container creation and forbids public-name re-resolution at that security boundary. On the current parent this is a dependent future-state invariant, not evidence that the preceding creation/provenance admission gates are already GREEN. It intentionally does not change production Rust/API/schema.

## Security invariant

After creation-bound identity and provenance/P0-state admission establish one immutable backend network authority, all later network security or destructive authority must be carried by that ID or an equivalent private capability derived from it. The public correlation may remain in consumer-visible lease/audit data, but it must not be promoted back into:

- effective `internal` / DNS state verification;
- effective attachment proof;
- partial-launch cleanup;
- explicit termination;
- recovery deletion authority.

A later inspection may corroborate current state, but its selector must be the admitted object identity rather than a public name that can resolve to a different object.

## Minimum GREEN direction

Do not repair this by adding another expected-name predicate. First repair #127 so the creation operation is bound to immutable identity and the same object is provenance-admitted with expected generated name, canonical ID, `internal=true`, and DNS disabled. One atomic creation receipt/API may satisfy both parent obligations if its exact contract is independently verified; otherwise both parent REDs remain separate gates.

Then carry that admitted immutable network authority through the private launch lifecycle and into effective isolation verification. If post-start network state is inspected, address the admitted full ID. Effective container attachment must also match that same ID before readiness. Cleanup/termination remain exact-ID and non-force under their existing owner REDs.

If parent #127 replaces CLI name-based acquisition with a creation-bound backend API, adapt this RED to the new private capability without preserving obsolete call shape. The invariant is continuity of one creation-bound, provenance-admitted immutable authority through effective verification and lifecycle cleanup.

## Sequencing

#142 is dependent on #127 and must remain Draft while parent exact `e9ad58c... / 35598340133` is unexecuted. Parent classification/repair order is:

1. execute and classify the create→first-inspect creation-bound identity RED;
2. execute and classify the existing identity-provenance RED requiring expected name, canonical ID, `internal=true`, and DNS disabled before container creation;
3. apply the minimum parent mechanism that satisfies both obligations, or one atomic mechanism demonstrably covering both;
4. ordinary/non-force adapt #142 if the backend mechanism changed, preserving this post-admission continuity invariant;
5. execute #142 on that dependency-safe parent before any post-admission GREEN is claimed.

A GREEN or causal classification of only the create→first-inspect witness is insufficient if provenance admission remains RED. No predecessor result is promoted to the child exact.

#141 remains later no-admitted-ID orphan recovery and does not authorize name-based launch-time cleanup. Public correlation/private authority separation in successful leases remains unchanged.

## Decision record

**Problem.** The adapter currently obtains a canonical-format network ID, while parent provenance admission remains unresolved; even after that prerequisite is repaired, the effective-verification function boundary drops the network ID and re-resolves a public correlation for P0 state.

**Constraint.** Preserve the public lease correlation, current DDD ownership, fail-closed P0 isolation, exact-container identity, non-force cleanup, and parent #127 causal ordering without promoting a syntactically valid ID to stronger ownership evidence than the parent has actually proved.

**Rejected alternative.** Rechecking the generated name is not sufficient: a same-name replacement can present valid `internal=true` / DNS-disabled evidence. Randomer names reduce collision probability but do not establish object continuity. Requiring matching state without matching immutable identity proves configuration, not ownership. Treating canonical ID syntax alone as provenance admission also collapses two independent parent REDs.

**Selected direction.** First establish creation-bound and provenance-admitted immutable network authority on #127; then preserve that same private authority through effective isolation verification and later lifecycle operations.

**Effect.** A network object that merely wins the public name after admission cannot supply evidence for the exact container's isolation boundary, and #142 cannot false-GREEN on a parent that never established the ownership/provenance premise it depends on.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-rm.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
