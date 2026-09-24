# Podman network creation receipt decision

Status: Proposed owner mechanism for Draft #127. This document narrows the executed creation-bound identity RED; it does not claim GREEN or release readiness.

## Problem

Draft #127 exact `67095685df0e03530f78ad11e2e87ee427692c27` creates an invocation-local Podman network and then performs a separate `podman network inspect <generated-name>` to learn the backend network ID. Native CI `35721035315` executed the hostile same-name replacement witness and confirmed that a syntactically valid ID learned only through that later public-name lookup can be promoted to private attachment authority. Effective-attachment checks are already GREEN; the remaining defect is the origin of the immutable network identity.

The fix must bind attachment and cleanup authority to the object created by this invocation. Random-name strength, an additional predicate on the later name lookup, or a second name lookup does not establish provenance.

## Primary-source findings

Podman v6.0.0 contains a useful distinction between its CLI presentation and its underlying create operation.

The `podman network create` command calls `ContainerEngine.NetworkCreate`, receives the complete created network response, and then prints only `response.Name`. The public CLI therefore discards the created object's ID before returning to this adapter.

Inside `ContainerEngine.NetworkCreate`, Podman receives the concrete network object from the network backend, synchronously calls `NewNetworkEvent(events.Create, network.Name, network.ID, network.Driver)`, and only then returns the network object. `NewNetworkEvent` writes a network event whose `Network` field is the network name and whose `ID` field is the immutable backend ID. The v6.0.0 event schema exposes `ID`, `Network`, `Status`, `Time`, and `Type`.

The event-history time contract is also explicit in v6.0.0. `generateEventFilters` parses `--since` and `--until` with `util.ParseInputTime`; the resulting predicates accept an event only when `Event.Time.After(since)` and `Event.Time.Before(until)`. `ParseInputTime` accepts RFC3339/RFC3339Nano and fractional Unix timestamps. The journald eventer persists the event's own timestamp as RFC3339Nano `PODMAN_TIME` and reconstructs `Event.Time` from that value during reads, so the history filter is evaluating the creation event's recorded timestamp rather than minting a later read-time timestamp.

For QSR's CLI Anti-Corruption Layer, absolute fractional Unix seconds are the selected bound representation. They are accepted directly by Podman v6.0.0, can preserve nanosecond precision without adding a date/time dependency, and make the invocation-local clock invariant executable in the focused RED. This is an adapter-local transport choice, not a public application-service or domain contract.

The Libpod REST `CreateNetwork` handler is also creation-bound: it calls the same `NetworkCreate` operation and returns its complete report. That is a valid transport-level receipt in principle, but switching QSR from the existing direct Podman CLI boundary to a REST socket adds service/socket availability, permission, lifecycle, and Podman-machine/Colima portability obligations. The hidden `podman system dial-stdio` command can proxy the configured connection, but upstream marks it as a hidden command that should not be invoked manually, and QSR's bounded subprocess adapter currently has no stdin transport contract. Neither is the minimum causal change for this owner lane.

## Selected mechanism

Use the Podman network-create event as an equivalent creation-bound receipt while retaining the existing CLI transport.

For one launch:

1. Record a narrow real wall-clock lower bound immediately before invoking `podman network create`, encoded for this ACL as absolute Unix seconds with up to nine fractional decimal digits.
2. Run the existing network create command without `--ignore`.
3. Record an upper wall-clock bound immediately after successful return using the same representation. The upper bound must be strictly later than the lower bound.
4. Query non-streaming Podman event history exactly once for `Type=network` and `Status=create` within that bounded interval. Because Podman applies strict `After(since)` / `Before(until)` predicates, the bounds must enclose rather than equal the creation event timestamp.
5. Admit a receipt only when exactly one event matches all of: `Type=network`, `Status=create`, exact invocation-generated network name, and one canonical lower-case 64-hex backend ID.
6. Use only that event ID as private attachment and cleanup authority. Verify the created object's P0 network state through an exact-ID inspect before container creation.
7. If the event backend is disabled, the event is absent, history is unavailable/rotated, the result is malformed, or more than one matching create event is present, fail closed. Do not fall back to public-name identity lookup and do not delete by public name.

This mechanism closes the executed TOCTOU because the event carrying the ID is emitted by the same Podman `NetworkCreate` operation before the CLI returns. A replacement created after the original create can generate another event, but it cannot rewrite the original event's ID. Ambiguous multiple matches fail closed. If the original object has already disappeared, the subsequent exact-ID inspection fails closed rather than promoting a replacement that happens to own the same name.

## Rejected shortcuts and alternatives

A post-create `network inspect <name>` with additional `name`, `internal`, or DNS predicates is rejected: a replacement can satisfy the same self-consistent fields.

A direct Libpod REST create response is not rejected architecturally, but it is deferred because this PR has not yet proved the rootless service/socket contract or Podman-machine/Colima parity required for a transport substitution.

`podman system dial-stdio` is deferred because it is an upstream hidden command, explicitly described as not for manual invocation, and using it safely would also require a bounded bidirectional HTTP transport implementation in QSR.

Treating high-entropy `qsr-net-*` names as authority is rejected. Entropy lowers accidental collision probability; it does not make a mutable name an immutable ownership receipt.

Using arbitrary historical or future event-history bounds is rejected even when they are syntactically valid. Such a query can admit unrelated same-name create events and therefore does not prove that the selected receipt belongs to this invocation. The focused witness must prove that the concrete lower/upper values are wall-clock timestamps captured inside the enclosing launch execution, not merely distinct option operands.

## RED and acceptance

`tests/podman_application_service_network_creation_receipt_red.rs` models one successful network-create operation whose creation history identifies `aaaaaaaa…` while a later same-name lookup resolves a replacement `bbbbbbbb…`. Acceptance requires QSR to issue exactly one bounded non-streaming JSON creation-history query, use concrete invocation-local wall-clock bounds, bind container creation to the creation-event ID, avoid minting authority from a post-create public-name lookup, and retain the same creation ID for partial cleanup.

The witness also models ambiguous history with two matching create events carrying different canonical IDs. That state must fail before container creation and must not mint destructive cleanup authority. Query-shape controls require exact stream/format/filter tokens; launch-clock controls require the lower/upper values to parse as absolute fractional Unix timestamps inside the enclosing `launch_at` interval and to be strictly ordered. These controls exist to prevent a malformed or over-broad query from false-GREENing the provenance repair.

The existing `podman_application_service_network_create_inspect_toctou_red` remains valid. The new witness does not replace it; it narrows the minimum owner-safe mechanism that can make that causal RED GREEN.

After implementation, exact-head validation must still include the existing effective-attachment hostile cases, non-force exact-ID cleanup, foreign-member safety, #141 durable orphan recovery, #142 post-admission continuity, applicable real rootless Podman evidence, and positive selected-LSM evidence. Event-journal unavailability is an availability failure, not permission to weaken provenance.

## DDD boundary

Creation-receipt admission remains inside `infrastructure::podman`, the Podman Anti-Corruption Layer. The application-service domain receives only an admitted private backend identity or a typed fail-closed error. Public lease `network_id` remains correlation/evidence; it must not be used to reconstruct destructive authority.

## References

Podman Authors. (2026). *podman network create implementation* (v6.0.0, `cmd/podman/networks/create.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/cmd/podman/networks/create.go

Podman Authors. (2026). *Network ABI implementation* (v6.0.0, `pkg/domain/infra/abi/network.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/domain/infra/abi/network.go

Podman Authors. (2026). *Libpod network API handlers* (v6.0.0, `pkg/api/handlers/libpod/networks.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/api/handlers/libpod/networks.go

Podman Authors. (2026). *Libpod event creation* (v6.0.0, `libpod/events.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events.go

Podman Authors. (2026). *Event schema* (v6.0.0, `libpod/events/config.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events/config.go

Podman Authors. (2026). *Event filtering implementation* (v6.0.0, `libpod/events/filters.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events/filters.go

Podman Authors. (2026). *Input-time parsing utility* (v6.0.0, `pkg/util/utils.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/pkg/util/utils.go

Podman Authors. (2026). *Journald event implementation* (v6.0.0, `libpod/events/journal_linux.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/libpod/events/journal_linux.go

Podman Authors. (2026). *Podman system dial-stdio implementation* (v6.0.0, `cmd/podman/system/dial_stdio.go`). Podman. https://github.com/containers/podman/blob/v6.0.0/cmd/podman/system/dial_stdio.go
