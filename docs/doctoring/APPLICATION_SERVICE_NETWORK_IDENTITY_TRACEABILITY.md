# Application-service network identity traceability

## Problem

The P0 application-service profile requires one runtime-owned internal, DNS-disabled network and fail-closed cleanup. The current Podman adapter creates a generated `qsr-net-*` name and then keeps using that name as the selector for container attachment, inspection, and cleanup. Successful `podman network create` output does not provide an immutable ownership identifier: Podman documents that creation displays the newly added network **name**. `podman network inspect` separately exposes the network `.ID` and `.Name`.

A generated name is therefore correlation metadata, not sufficient backend lifecycle authority. If that name is replaced or rebound after successful creation, a later name lookup can select a different same-principal network while the runtime's originally created network remains orphaned.

Issue #48 owns this network-lifecycle identity gap. It is independent of #22/#23 effective attachment, #41 network-level force-removal authority, #20 generated-name uniqueness, #40 acquired **container** identity, and #42 serialized-lease cleanup authorization.

## Constraints

- Preserve the public P0 contract: per-sandbox internal network, DNS disabled, loopback-only publication, no arbitrary egress.
- Do not move network identity selection to the caller or a deserialized lease.
- Do not treat a larger random name, a label, or create-plan argv as immutable ownership proof.
- Do not restore `podman network rm --force`; #41 requires foreign members to survive cleanup uncertainty.
- Keep backend-applied configuration evidence distinct from real negative-egress/runtime evidence.
- Preserve generated `qsr-net-*` names as public/audit correlation metadata unless a versioned contract deliberately changes them.

## Primary evidence

Podman documents `podman network create [options] [name]` and states that successful creation displays the name of the new network. The same documentation exposes `--ignore` as name-based existence behavior, reinforcing that name lookup and resource identity are distinct concerns.

Podman network inspection exposes `.ID`, `.Name`, `.Internal`, `.DNSEnabled`, labels, and attached containers. The documented example shows a full 64-hex network ID separate from the human-readable network name. Podman's container networking accepts a user-defined network by either network name or ID.

NIST SP 800-190 treats the container runtime and its network/resource controls as part of the isolation boundary. For this runtime, ownership of destructive and attachment selectors is therefore evidence that must be bound to the exact resource created for the invocation rather than inferred from a mutable name.

## RED authority

`tests/podman_application_service_network_identity_red.rs` on Draft #23 models an otherwise-positive launch. Fake Podman exposes an invocation-owned full network ID, but any container creation that re-resolves the generated network name records a foreign-network side effect. The RED requires:

- network identity inspection before container creation;
- container `--network` selection by the acquired network ID;
- preservation of the public generated network name as correlation metadata;
- no foreign-network marker mutation.

Current production is expected to fail because `launch_at` creates the network and immediately creates the container with `--network <qsr-net-name>`; its first network inspection occurs only after the container has started.

## Smallest causal GREEN after executed RED

1. Treat successful network creation as the start of an ownership-acquisition phase, not as proof that later name resolution selects the same object.
2. Inspect the just-created name immediately and parse exactly one well-formed full network ID plus exact expected name and P0 internal/DNS state.
3. Use the acquired ID as the container network selector where Podman supports an ID selector.
4. Carry the acquired network identity through effective attachment verification and internal runtime cleanup provenance.
5. Before destructive removal, use the ID when supported by the reviewed Podman command; otherwise prove the name still resolves to the same acquired ID immediately before a non-force removal.
6. Fail closed on missing, malformed, multiple, changed, or ambiguous identity evidence. Cleanup uncertainty must not widen destructive authority.

This first GREEN establishes backend resource identity binding. It does not by itself prove negative egress or kernel-level isolation; those remain real-runtime release evidence.

## Alternatives rejected

- **Rely only on collision-resistant generated names.** This reduces accidental collision risk but does not prevent stale-resource or post-create name re-resolution.
- **Trust network-create stdout as an ID.** Podman documents that the command displays the newly created network name.
- **Use a network label as sole authority.** A same-principal backend actor can create or mutate similarly labeled resources; labels are useful provenance, not immutable identity by themselves.
- **Use `network rm --force`.** #41 demonstrates that this delegates deletion of foreign attached containers and is broader than sandbox ownership.
- **Store the selector only in the public lease.** #42 establishes that serializable evidence must not become sufficient destructive backend authority.

## Expected effect

The application-service network lifecycle becomes symmetric with the container lifecycle boundary: generated names remain human/audit correlation values, while backend operations are tied to an identity acquired from the created resource. A same-name replacement can no longer silently capture container attachment or cleanup authority.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-run — Run a command in a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-run.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
