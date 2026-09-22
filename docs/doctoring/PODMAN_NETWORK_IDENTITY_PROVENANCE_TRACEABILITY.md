# Podman network identity provenance traceability

## Current status

Draft #127 has execution-backed evidence that the current production sequence does not establish creation-bound network authority.

Production still creates an internal network under an invocation-local `qsr-net-*` correlation name and then calls `podman network inspect --format json <generated-name>` to learn a canonical 64-character lower-case Podman network ID. Container creation is rewritten to use that ID, later effective attachment is checked against the acquired ID, and partial-launch cleanup already removes the acquired ID without network-level force semantics.

The remaining creation defect is earlier. A public network name is mutable resolution state. If the object created by this invocation disappears or is replaced before the first name lookup, a conforming same-name replacement can provide a different canonical ID while preserving the expected name, `internal=true`, and DNS-disabled state. Promoting that later lookup result to private attachment or cleanup authority is a TOCTOU ownership error.

Exact `3a94ef6cc158bd12a47c5fb7701e2158d7276341` / native CI `35778989881` executed this causal RED on GitHub-hosted runners. Coverage reached `podman_application_service_network_create_inspect_toctou_red::first_name_inspect_cannot_promote_a_same_name_replacement_network` after existing create-receipt, acquired-ID, effective-attachment, and partial-cleanup controls. The failure was therefore the intended creation-provenance defect rather than a fixture prerequisite. Hosted rootless/AppArmor negative acceptance was also GREEN. Verify independently stopped at deterministic formatting deltas in the hardened creation-receipt witness; formatter-only successor `5e4490c03cb39b28ad5a799e817fded81f756393` changed no production semantics.

## Creation-bound receipt decision

The minimum selected repair remains inside the existing Podman CLI Anti-Corruption Layer. It does not treat the public correlation name as authority and does not add a new service/API dependency.

For one launch invocation the adapter must:

1. capture an absolute wall-clock lower bound immediately before the non-`--ignore` `podman network create` call;
2. capture the upper bound immediately after successful create return;
3. issue exactly one non-streaming JSON `podman events` history query bounded by those invocation-local timestamps and filtered exactly to `type=network` plus `event=create`;
4. admit private authority only when exactly one matching create event has `Network` equal to the generated correlation name and `ID` equal to one canonical lower-case 64-hex Podman ID;
5. fail closed on unavailable, disabled, rotated, malformed, missing, or ambiguous creation history without public-name identity or deletion fallback;
6. inspect the admitted object by that exact ID and require the same generated name plus `internal=true` and DNS disabled before container creation;
7. bind container `--network` to the exact admitted ID and retain that same ID for partial-launch cleanup and later private lifecycle succession.

Podman 6.0.0 provides the evidence needed for this choice. The CLI `network create` path receives the created network object but prints only its name. Inside the same create operation, Podman emits a network-create event carrying the concrete network `ID` and `Network` name before returning. Event history supports bounded `since`/`until` filtering; QSR's focused witness uses absolute fractional Unix seconds as the adapter-local representation so the bounds can be tied to the enclosing launch invocation without adding a date/time dependency.

Missing creation history is an availability failure, not permission to reconstruct ownership from a later name lookup.

## Provenance/P0 witness succession

The original `podman_application_service_network_identity_provenance_owner_red` predated the creation-receipt decision. It correctly required fail-closed rejection for three independent contradictions—wrong network name, external network state, and DNS enabled—but it encoded a later exact-name inspection as the expected acquisition path. That test shape became stale once the creation-provenance RED executed and the receipt mechanism was selected.

Review `5285160963` records the repair finding. Test-only exact `2637ffd98ee69649b082a171e20bff857b7fc6f0` preserves the invariant while replacing the obsolete transport shape:

- creation occurs before receipt admission;
- one creation receipt supplies the immutable candidate ID;
- P0 provenance/state inspection targets that exact ID;
- the exact-ID inspection must still report the expected generated network, canonical ID, `internal=true`, and DNS disabled;
- wrong name, external-network state, or DNS-enabled state fails before container creation;
- public-name identity inspection is explicitly forbidden.

The dedicated `podman_application_service_network_creation_receipt_red` remains the owner of exact event-query token grammar, invocation-local wall-clock bounds, same-name replacement, ambiguity, and fail-closed no-fallback behavior. The provenance/P0 witness intentionally does not duplicate those transport-shape assertions.

Current `2637ffd...` is test-only and does not make the production mechanism GREEN. Its native CI `35798043259` must execute independently; no predecessor status transfers.

## Ownership and DDD boundary

`application_service` remains the Supporting bounded context that owns runtime lifecycle and admitted private resource authority. Podman remains an infrastructure adapter behind the context ACL. Podman event/inspection DTOs are transport evidence, not domain contracts.

The generated `qsr-net-*` value remains audit and consumer-visible correlation metadata. It must not become attachment, P0 verification, cleanup, recovery, or termination authority merely because it is unpredictable or currently resolves to an object.

Creation receipt admission and P0 state are separate checks on the same object:

- the receipt proves which immutable object this invocation created;
- exact-ID inspection corroborates that object's expected name and deny-by-default network state;
- effective container inspection later proves the acquired container is attached only to the same admitted ID;
- cleanup/recovery/termination must preserve the same private authority without network-level force semantics.

Dependent #142 owns post-admission continuity. It must not re-resolve the public name when later effective-isolation evidence is collected. #141 remains the durable no-admitted-ID recovery boundary. Successful-lease private cleanup authority and foreign-safe non-force termination remain downstream gaps after creation-bound admission is GREEN.

## Rejected alternatives

Larger or more random names reduce accidental collision probability but do not establish object identity. More predicates on the later public-name inspection still allow a conforming same-name replacement to win the race. Runtime labels remain corroborating mutable metadata rather than an immutable creation receipt. Repeated name inspection widens the race instead of closing it.

Libpod REST create can return creation-bound object data, but switching transports would also introduce rootless service/socket lifecycle, permission, failure-semantics, and Podman-machine/Colima portability obligations. Hidden `podman system dial-stdio` has the same transport-boundary problem and is not the minimum repair. Both remain deferred until independent evidence justifies that larger adapter change.

## Next executable gate

Execute the current test/doctoring successor unchanged. Once the adapted provenance/P0 witness is causally classified, implement only the bounded creation-event receipt and exact-ID P0 admission in `src/infrastructure/podman.rs`. Then rerun the unchanged same-name replacement, ambiguous-history, exact-ID provenance/P0, effective-attachment, and exact-ID/non-force partial-cleanup witnesses.

A qualifying GREEN still requires repository validation, rustfmt, locked workspace/all-target tests, Clippy and public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, real rootless runtime evidence, positive selected-LSM evidence, qualifying review/thread/security gates, protected integration, and immutable release evidence. No mutable PR head or public correlation name is release authority.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-events — Monitor Podman events* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-events.1.html

Podman Authors. (2026). *Network ABI implementation* (v6.0.0, `pkg/domain/infra/abi/network.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/domain/infra/abi/network.go

Podman Authors. (2026). *Libpod event creation* (v6.0.0, `libpod/events.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events.go

Podman Authors. (2026). *Event schema and filtering implementation* (v6.0.0, `libpod/events`). Podman. https://github.com/containers/podman/tree/v6.0.0/libpod/events

Podman Authors. (2026). *Input-time parsing utility* (v6.0.0, `pkg/util/utils.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/util/utils.go

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-inspect.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
