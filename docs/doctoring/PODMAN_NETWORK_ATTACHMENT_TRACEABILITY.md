# Podman Network Attachment Traceability

Last reviewed: 2026-09-16 KST

## Scope

This note records the evidence boundary for the canonical application-service network-lifecycle successor. It covers the relation between a runtime-selected network reference, Podman container-inspect evidence, the consumer-visible network correlation, and the stable network identity used for destructive authority. It does not promote a candidate repair to GREEN before current exact-head execution.

## Problem

The explicit-termination hostile fixture must launch successfully under both the historical correlation-name selector and the intended acquired-network-ID selector so that its first relevant failure remains destructive cleanup of an in-use owned network.

Predecessor exact `96d31affabc21b5915e3438f3de8ba1b62ef0e1d` recorded the selected `--network` argument and reused that value as both `HostConfig.NetworkMode` and the key of `NetworkSettings.Networks`. That is realistic while the selector is the generated `qsr-net-*` name, but it becomes structurally wrong when the selector is the full Podman network ID. Podman's container-inspect model defines `NetworkSettings.Networks` as a map **from network name to network information**; the stable identity is carried separately by each entry's `NetworkID` field.

Podman's current network-create documentation also states that successful `podman network create` displays the newly added network's **name**, while network inspection exposes a distinct `.ID` in addition to `.Name`, `.Internal`, `.DNSEnabled`, and other configuration. Therefore network-create stdout is correlation/name evidence, not the full immutable network identity required for attachment comparison or destructive cleanup.

A second authority-separation defect exists in the current production construction path. `ApplicationServiceCleanupAuthority.network_id` is derived from the same `RuntimeLeaseMetadata.network_id` that becomes public `ApplicationServiceLease.network_id()`. If a later cleanup repair merely replaces that metadata field with Podman's full network ID, cleanup obtains exact destructive authority but the public lease silently stops carrying the generated `qsr-net-*` correlation. That would change consumer-visible semantics solely to satisfy an internal cleanup requirement.

A third defect remains after the first acquired-ID repair. `launch_at` now acquires a full Podman network ID and uses it to bind `container create`, but every partial-launch cleanup path still calls `cleanup_network(&plan)`. That helper discards the acquired identity and executes `podman network rm --force <generated qsr-net-*>`. The runtime therefore has exact destructive authority in hand and then abandons it precisely when cleanup starts. A re-resolved correlation name can consequently become a destructive target during failure handling.

Podman's current `network rm` documentation makes this particularly unsafe: `--force` removes all containers using the named network, stopping and removing running containers as necessary. Ordinary non-force removal instead reports an in-use network as a failure. The application-service cleanup contract therefore must not combine a re-resolvable generated name with network-level force removal.

## Executed owner-path RED

Canonical successor exact `323c5f834930ecabfa29831c4901599a5522a90d` executed in CI run `35035358796` on 2026-09-16. Both branch-coverage job `104603157091` and coverage job `104603157311` reached `tests/podman_application_service_network_acquired_id_owner_red.rs` after the existing application-service suites had passed. The test failed at the intended first chronology boundary: no `network inspect --format json <generated qsr-net-...>` occurred between network creation and container creation, so the runtime had no backend-acquired network identity before binding the container.

The same exact head's verify job `104603157449` independently stopped at `cargo fmt --check` on formatting-only deltas in the network-binding and missing-evidence owner tests. Ordinary descendants `6fa59c86334624689d5dc426b72632d25199b80c` and `4cbfaf6a2b5306ffefb9c5c3fd90e4ba9d8e92e1` apply exactly those formatter-prescribed layouts; they do not change the causal RED.

## First minimum causal repair

Production descendant `ea280bf6e10b83324dd0d4a6f8a82c213d9b06f7` repairs only the first executed boundary. After `network create`, the adapter now performs exact-name `network inspect --format json <qsr-net-...>`, requires exactly one inspection object, requires its `id` to be a canonical lowercase 64-hex backend identifier, and rewrites the already-generated container-create `--network` selector to that acquired ID before `container create`.

The generated `qsr-net-*` name remains the public correlation and current network-object inspection key. The repair does **not** yet claim effective container attachment verification, private cleanup-authority separation, or non-force network cleanup. Those remain subsequent causal boundaries. Existing production secure-default fields (`EffectiveCaps`, `BoundingCaps`, `dns_enabled`) also remain unchanged because the replayed missing-evidence test did not execute on `323c5f...`; the acquired-ID RED stopped the suite earlier.

The selector rewrite is itself fail closed: a missing or value-less internal `--network` argument yields `BackendInvocationFailed { operation: "container_create_network_selector" }`. Unit tests cover successful replacement plus both malformed internal shapes so the helper does not introduce an uncovered impossible branch.

Because pre-existing fake-Podman fixtures historically returned only `internal`/`dns_enabled` during network inspection, some fixtures may need to add a realistic `id` before the new source can achieve a current-head GREEN. Production must not retain a generated-name fallback merely to preserve those stale test doubles; backend identity evidence is the contract being repaired.

## Partial-launch cleanup authority RED

Test-only exact `7980b1ab6417db4cd7be895a874aa22ad1dcb4ee` adds `tests/podman_application_service_network_partial_cleanup_owner_red.rs` on the canonical #127 branch. It reaches the already-repaired sequence `network create -> exact-name network inspect -> ID-bound container create`, then makes `container create` fail before a trustworthy cidfile identity exists. At that point the fixture models the generated `qsr-net-*` correlation as resolving to an unrelated network.

The witness requires cleanup to retain the previously acquired Podman network ID, execute `network rm <OWNED_NETWORK_ID>` without network-level `--force`, preserve the unrelated correlation-name network, and then return the original `BackendCommandFailed { operation: "container_create" }`. A generated-name force cleanup mutates the foreign marker and therefore remains an active causal RED. Production was not changed in this commit, and the RED is not execution evidence until an exact-head CI job reaches it.

This closes a gap that successful explicit termination alone does not cover. Private cleanup authority is needed as soon as network identity has been acquired, including partial launch failures before any lease can be published. It cannot be reconstructed later from public lease metadata because no lease exists on these paths.

## Decision

Exact fixture repair `7bdb2fe626f0d10cb2560ae1218560f25f8ea46c` keeps the selected network reference only in `HostConfig.NetworkMode`, while `NetworkSettings.Networks` is keyed by the generated network name and carries `OWNED_NETWORK_ID` in `NetworkID`.

Test-only authority hardening `8a9790357f48812bbdb8a3de031a2ed27f521973` additionally records the successful lease's public `network_id()` and requires it to remain exactly the generated `qsr-net-*` correlation and to differ from `OWNED_NETWORK_ID`. The same termination witness still requires non-force cleanup to target `OWNED_NETWORK_ID`. This prevents a future production repair from conflating public correlation evidence with private destructive authority.

Once a network ID is acquired, every later failure and termination path must carry that ID as runtime-private authority. A pre-acquisition failure has no such authority and must fail closed without turning the generated correlation name into an equivalent destructive identifier. The minimum GREEN therefore needs one lifecycle-owned network authority value that survives from identity admission through partial cleanup, successful lease registration, and explicit termination; public correlation remains a separate value.

The contract therefore preserves five distinct concepts:

- `qsr-net-*`: invocation-local correlation, Podman network name, and current public lease network evidence;
- selected `--network` argument: historical name selector or acquired stable-ID selector;
- `NetworkSettings.Networks` key: effective attached network name as reported by Podman;
- `NetworkID`: stable effective network identity used to compare attachment evidence;
- private cleanup authority: exact acquired identity retained for destructive operations without being reconstructed from consumer-visible metadata.

## Rejected alternatives

- **Keep the selected selector as the map key.** Rejected because it makes ID-bound inspect evidence unlike Podman's documented data model and can move the RED to a fixture artifact.
- **Treat network-create stdout as the acquired network ID.** Rejected because Podman documents that `network create` prints the newly added network name; the stable network ID must be obtained from inspection rather than inferred from that output.
- **Fall back to the generated name when network inspection omits `id`.** Rejected because that converts missing destructive-identity evidence into affirmative authority and would make legacy test doubles stronger than the production backend contract.
- **Keep using `cleanup_network(&plan)` after acquired-ID admission.** Rejected because `PodmanLaunchPlan` carries only the generated name; dropping an already-admitted stable ID and force-removing a later resolution of the correlation name reintroduces the exact authority ambiguity the acquired-ID repair was meant to remove.
- **Require only the network name.** Rejected because the acquired-ID RED requires stable identity before container creation and the termination witness must remain compatible with that future GREEN.
- **Ignore `NetworkSettings.Networks` and trust `HostConfig.NetworkMode`.** Rejected because configuration intent is not effective attachment proof; the hostile contract requires exact effective membership before readiness.
- **Put the acquired Podman ID into public `ApplicationServiceLease.network_id()`.** Rejected because it changes an existing consumer-visible correlation field to satisfy an internal destructive-authority need. A public semantic change would require its own versioned contract decision rather than occurring as an incidental cleanup implementation detail.
- **Reconstruct cleanup authority from the generated name later.** Rejected because the name is re-resolvable correlation metadata rather than an immutable acquired resource identity and must not become destructive authority by lookup.
- **Use `podman network rm --force` as a cleanup guarantee.** Rejected because Podman explicitly defines force removal to remove containers using the named network. An in-use network must instead fail cleanup closed so foreign membership is preserved.

## Verification contract

A valid integrated GREEN must eventually demonstrate, on one exact head:

1. network creation by invocation-local `qsr-net-*` name;
2. exact-name inspection and acquisition of the full Podman `.ID` before container creation;
3. container binding by acquired ID;
4. any post-acquisition partial-launch failure retains that acquired ID for non-force exact network cleanup and cannot delete a resource reached only by the generated correlation name;
5. exact acquired-container inspection showing one effective attachment whose `NetworkID` equals the acquired identity and no additional network;
6. no readiness publication after attachment mismatch;
7. a successful lease whose public `network_id()` remains the generated correlation rather than the acquired destructive ID;
8. private cleanup authority retaining the exact acquired network identity independently of the public lease field;
9. explicit termination that removes only exact owned resources and performs non-force network cleanup so foreign membership returns `CleanupFailed` without deleting the foreign member.

The current candidate has implemented only items 2 and 3 after the executed RED. Exact-head CI must now determine the next real boundary; predecessor GREEN, network-object configuration alone, a fake-backend-only shape, or a public-field semantic substitution are not substitutes for that execution.

## Exact-head linkage

- Canonical application-service owner: PR #21 exact `65f69de6eb1cf78b316b38424f8c35c316cd0672`.
- Network-lifecycle successor: PR #127.
- Executed acquired-ID RED: `323c5f834930ecabfa29831c4901599a5522a90d`, run `35035358796`, jobs `104603157091` and `104603157311`.
- Formatting-only descendants: `6fa59c86334624689d5dc426b72632d25199b80c`, `4cbfaf6a2b5306ffefb9c5c3fd90e4ba9d8e92e1`.
- First production candidate repair: `ea280bf6e10b83324dd0d4a6f8a82c213d9b06f7`.
- Earlier fixture name/identity-shape repair: `7bdb2fe626f0d10cb2560ae1218560f25f8ea46c`.
- Earlier public-correlation/private-authority hardening: `8a9790357f48812bbdb8a3de031a2ed27f521973`.
- Partial-launch cleanup-authority RED: `7980b1ab6417db4cd7be895a874aa22ad1dcb4ee`.
- Related tests:
  - `tests/podman_application_service_network_acquired_id_owner_red.rs`
  - `tests/podman_application_service_network_binding_owner_red.rs`
  - `tests/podman_application_service_network_termination_owner_red.rs`
  - `tests/podman_application_service_network_partial_cleanup_owner_red.rs`
  - `tests/podman_missing_isolation_evidence_owner_red.rs`

## References

The Podman Project. (2026). *Container inspection data structures* [Source code]. GitHub. https://github.com/containers/podman/blob/main/libpod/define/container_inspect.go

The Podman Project. (2026). *podman-inspect — Display artifact, container, image, volume, network, or pod configuration* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-inspect.1.html

The Podman Project. (2026). *podman-network-create — Create a Podman network* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-create.1.html

The Podman Project. (2026). *podman-network-inspect — Display the network configuration for one or more networks* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

The Podman Project. (2026). *podman-network-rm — Remove one or more networks* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html
