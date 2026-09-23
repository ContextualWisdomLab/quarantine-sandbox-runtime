# Podman network identity provenance traceability

## Current status

Draft #127 has execution-backed evidence that the predecessor production sequence did not establish creation-bound network authority.

Exact `3a94ef6cc158bd12a47c5fb7701e2158d7276341` / native CI `35778989881` executed the causal RED on GitHub-hosted runners. Coverage reached `podman_application_service_network_create_inspect_toctou_red::first_name_inspect_cannot_promote_a_same_name_replacement_network` after existing create-receipt, acquired-ID, effective-attachment, and partial-cleanup controls. The failure was therefore the intended creation-provenance defect: production created an internal network under an invocation-local `qsr-net-*` correlation name and then called `podman network inspect --format json <generated-name>` to mint private authority from mutable name resolution. Hosted rootless/AppArmor negative acceptance was also GREEN. Verify independently stopped at deterministic formatting deltas in the hardened creation-receipt witness; formatter-only successor `5e4490c03cb39b28ad5a799e817fded81f756393` changed no production semantics.

Review `5285402062` then found one stale positive fixture in `podman_application_service_network_creation_receipt_red`: its exact-ID P0 inspection returned a synthetic `created-object` name even though the sibling provenance/P0 contract requires that the admitted ID still identify the invocation's generated `qsr-net-*` object. Test-only `709d574107f6cb0f024a5780e6e89920292f37a9` repairs only that fixture and preserves the same-name replacement, ambiguity, bounded-events, exact-ID attachment, and cleanup assertions.

Production exact `0e44b88b7ecf72a5c641b050a7d8450d604da4e4` introduced the minimum selected creation-receipt mechanism in `src/infrastructure/podman.rs`. Review `5285457161` then found one execution-fidelity defect in its boolean argv: Podman/pflag requires the explicit false form `--stream=false`; a detached `--stream`, `false` pair can set the no-value boolean form and leave `false` as a positional argument. Production successor `421b8913cd4f74a0728a138f856409de1700fcb6` changes only that argv spelling. The mechanism is not GREEN until one unchanged integrated exact executes all repository gates; no predecessor status transfers.

## Creation-bound receipt decision

The repair remains inside the existing Podman CLI Anti-Corruption Layer. It does not treat the public correlation name as authority and does not add a new service/API dependency.

For one launch invocation the adapter must:

1. capture an absolute wall-clock lower bound immediately before the non-`--ignore` `podman network create` call;
2. capture the upper bound immediately after successful create return;
3. issue exactly one non-streaming JSON `podman events` history query bounded by those invocation-local timestamps and filtered exactly to `type=network` plus `event=create`;
4. admit private authority only when exactly one matching create event has `Network` equal to the generated correlation name and `ID` equal to one canonical lower-case 64-hex Podman ID;
5. fail closed on unavailable, disabled, rotated, malformed, missing, or ambiguous creation history without public-name identity or deletion fallback;
6. inspect the admitted object by that exact ID and require the same generated name plus `internal=true` and DNS disabled before container creation;
7. bind container `--network` to the exact admitted ID and retain that same ID for partial-launch cleanup and later private lifecycle succession.

Podman 6.0.0 provides the evidence needed for this choice. The CLI `network create` path receives the created network object but prints only its name. Inside the same create operation, Podman emits a network-create event carrying the concrete network `ID` and `Network` name before returning. Event history supports bounded `since`/`until` filtering; QSR uses absolute fractional Unix seconds as the adapter-local representation so the bounds are tied to the enclosing launch invocation without adding a date/time dependency. Podman's own event tests and operator examples use `--stream=false` for a non-streaming boolean value, which is why `421b891...` keeps that flag in assignment form while leaving `--format`, bounds, and filters as ordinary value-bearing options.

Missing creation history is an availability failure, not permission to reconstruct ownership from a later name lookup.

## Production candidate behavior

Current production candidate `421b8913cd4f74a0728a138f856409de1700fcb6`, building on `0e44b88...`, changes only the Podman infrastructure path after the fixture prerequisite at `709d574...`:

- `SystemTime` bounds are captured immediately before and after successful `network create`;
- the adapter calls `podman events --stream=false --format json --since <lower> --until <upper> --filter type=network --filter event=create` exactly once;
- JSON Lines are parsed as Podman network-create events, unrelated network names may be ignored, and missing, malformed, non-network/non-create, invalid-ID, or multiple matching receipts fail closed;
- the matching event ID must be canonical lower-case 64-hex;
- P0 inspection is issued against that exact ID, and the returned object must carry the same ID, the invocation's generated name, `internal=true`, and DNS disabled;
- once the exact receipt ID exists, P0 inspection/parse/contradiction failure attempts non-force cleanup by that ID rather than falling back to the public name;
- container `--network`, effective attachment verification, and partial-launch cleanup continue to use the admitted ID.

The candidate deliberately does not change successful-lease metadata or explicit termination. Lease metadata still carries public correlation and termination still has network-level `--force`; those remain downstream lifecycle defects owned by the existing #142/#141 succession rather than being folded into this causal creation repair.

## Provenance/P0 witness succession

The original `podman_application_service_network_identity_provenance_owner_red` predated the creation-receipt decision. It correctly required fail-closed rejection for three independent contradictions—wrong network name, external network state, and DNS enabled—but it encoded a later exact-name inspection as the expected acquisition path. That test shape became stale once the creation-provenance RED executed and the receipt mechanism was selected.

Review `5285160963` records the repair finding. Test-only exact `2637ffd98ee69649b082a171e20bff857b7fc6f0` preserves the invariant while replacing the obsolete transport shape:

- creation occurs before receipt admission;
- one creation receipt supplies the immutable candidate ID;
- P0 provenance/state inspection targets that exact ID;
- the exact-ID inspection must still report the expected generated network, canonical ID, `internal=true`, and DNS disabled;
- wrong name, external-network state, or DNS-enabled state fails before container creation;
- public-name identity inspection is explicitly forbidden.

The dedicated `podman_application_service_network_creation_receipt_red` remains the owner of exact event-query token grammar, invocation-local wall-clock bounds, same-name replacement, ambiguity, and fail-closed no-fallback behavior. Review `5285402062` and `709d574...` align its positive exact-ID P0 fixture with the same generated-name invariant rather than weakening either witness.

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

Execute the current integrated #127 successor unchanged. The first required result is that the hardened creation-receipt and provenance/P0 witnesses move from the executed predecessor RED to exact-head GREEN without public-name fallback, and that the canonical `--stream=false` invocation is accepted by real/fake Podman paths. Any valid-path fixture exposed by the new required event-history call must be migrated to the same creation-receipt contract rather than weakening production or adding a name fallback.

Then rerun the unchanged same-name replacement, ambiguous-history, exact-ID provenance/P0, effective-attachment, and exact-ID/non-force partial-cleanup witnesses together with repository validation, rustfmt, locked workspace/all-target tests, Clippy and public/private rustdoc with warnings denied, complete applicable owned-production statement/function/region/branch/edge coverage, real rootless runtime evidence, positive selected-LSM evidence, qualifying review/thread/security gates, protected integration, and immutable release evidence. No mutable PR head or public correlation name is release authority.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-events — Monitor Podman events* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-events.1.html

Podman Authors. (2026). *Network ABI implementation* (v6.0.0, `pkg/domain/infra/abi/network.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/domain/infra/abi/network.go

Podman Authors. (2026). *Libpod event creation* (v6.0.0, `libpod/events.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events.go

Podman Authors. (2026). *Event schema and filtering implementation* (v6.0.0, `libpod/events`). Podman. https://github.com/containers/podman/tree/v6.0.0/libpod/events

Podman Authors. (2026). *Input-time parsing utility* (v6.0.0, `pkg/util/utils.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/util/utils.go

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-inspect.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
