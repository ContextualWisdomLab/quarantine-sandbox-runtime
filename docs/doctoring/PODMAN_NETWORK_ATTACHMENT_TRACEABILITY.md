# Podman Network Attachment Traceability

Last reviewed: 2026-09-16 KST

## Scope

This note records the evidence boundary for the canonical application-service network-lifecycle successor. It covers only the relation between a runtime-selected network reference, Podman container-inspect evidence, and the stable network identity used for destructive authority. It does not promote the current test-only branch to GREEN and does not authorize production networking changes before exact-head RED execution.

## Problem

The explicit-termination hostile fixture must launch successfully under both the current correlation-name selector and the intended acquired-network-ID selector so that its first relevant failure remains destructive cleanup of an in-use owned network.

Predecessor exact `96d31affabc21b5915e3438f3de8ba1b62ef0e1d` recorded the selected `--network` argument and reused that value as both `HostConfig.NetworkMode` and the key of `NetworkSettings.Networks`. That is realistic while the selector is the generated `qsr-net-*` name, but it becomes structurally wrong when the selector is the full Podman network ID. Podman's container-inspect model defines `NetworkSettings.Networks` as a map **from network name to network information**; the stable identity is carried separately by each entry's `NetworkID` field.

A future acquired-ID implementation could therefore be rejected by a correct attachment verifier before the fixture reaches its intended explicit-termination boundary merely because the fake backend emitted an impossible name/identity shape.

## Decision

Exact repair `7bdb2fe626f0d10cb2560ae1218560f25f8ea46c` keeps the selected network reference only in `HostConfig.NetworkMode`, while `NetworkSettings.Networks` is keyed by the generated network name and carries `OWNED_NETWORK_ID` in `NetworkID`.

This preserves four distinct concepts:

- `qsr-net-*`: invocation-local correlation and Podman network name;
- selected `--network` argument: current name selector or future stable-ID selector;
- `NetworkSettings.Networks` key: effective attached network name as reported by Podman;
- `NetworkID`: stable effective network identity used to compare attachment evidence and to derive private cleanup authority.

The repair is test-only. Production Rust/API/schema/runtime semantics are unchanged.

## Rejected alternatives

- **Keep the selected selector as the map key.** Rejected because it makes future ID-bound inspect evidence unlike Podman's documented data model and can move the RED to a fixture artifact.
- **Require only the network name.** Rejected because the acquired-ID RED separately requires stable identity before container creation and the termination witness must remain compatible with that future GREEN.
- **Ignore `NetworkSettings.Networks` and trust `HostConfig.NetworkMode`.** Rejected because configuration intent is not effective attachment proof; the hostile contract requires exact effective membership before readiness.

## Verification contract

Current exact-head acceptance requires normal CI execution of the unchanged hostile contracts. A valid future GREEN must demonstrate, on one exact head:

1. network creation by invocation-local name;
2. exact-name inspection and acquisition of the full Podman `.ID` before container creation;
3. container binding by acquired ID;
4. exact acquired-container inspection showing one effective attachment whose `NetworkID` equals the acquired identity and no additional network;
5. no readiness publication after attachment mismatch;
6. explicit termination that removes only exact owned resources and performs non-force network cleanup so foreign membership returns `CleanupFailed` without deleting the foreign member.

Queued or cancelled runs, predecessor GREEN, network-object configuration alone, and a fake-backend-only shape are not substitutes for exact-head execution and applicable positive runtime evidence.

## Exact-head linkage

- Canonical application-service owner: PR #21 exact `65f69de6eb1cf78b316b38424f8c35c316cd0672`.
- Network-lifecycle successor: PR #127.
- Fixture predecessor: `96d31affabc21b5915e3438f3de8ba1b62ef0e1d`.
- Name/identity-shape repair: `7bdb2fe626f0d10cb2560ae1218560f25f8ea46c`.
- Related tests:
  - `tests/podman_application_service_network_acquired_id_owner_red.rs`
  - `tests/podman_application_service_network_binding_owner_red.rs`
  - `tests/podman_application_service_network_termination_owner_red.rs`

## References

The Podman Project. (2026). *Container inspection data structures* [Source code]. GitHub. https://github.com/containers/podman/blob/main/libpod/define/container_inspect.go

The Podman Project. (2026). *podman-inspect — Display artifact, container, image, volume, network, or pod configuration* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-inspect.1.html
