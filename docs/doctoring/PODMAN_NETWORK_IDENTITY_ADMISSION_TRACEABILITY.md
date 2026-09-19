# Podman network identity admission and cleanup authority traceability

## Problem

The application-service Podman adapter now creates a per-invocation `qsr-net-*` network, inspects that exact generated name, admits a canonical 64-character lower-case Podman network ID, and binds container creation to the acquired ID. The generated name is therefore correlation data; the admitted ID is the first stable backend identity suitable for destructive network authority.

A distinct failure path remains before identity admission. If network creation succeeds but exact-name inspection is malformed or does not yield an admissible ID, `launch_at` currently calls `cleanup_network(&plan)`. That helper resolves the generated `qsr-net-*` name again and executes `podman network rm --force <name>`. At precisely the point where the runtime has failed to prove which backend network object it owns, a re-resolvable correlation value is promoted to destructive authority.

This is not the same defect as the post-acquisition cleanup gap covered by `podman_application_service_network_partial_cleanup_owner_red.rs`. That later gap discards an ID that was already admitted. This document covers the earlier state in which no exact network identity has been admitted at all.

## Current evidence

Test-only commit `eda111120e2d11b8ff35c8de187053db2c9d0f2b` adds `tests/podman_application_service_network_identity_failure_cleanup_owner_red.rs`. The fake backend:

- accepts rootless/security prerequisite inspection;
- creates the generated network;
- returns malformed network identity evidence from exact-name inspection;
- would accept `network rm` if the runtime attempts destructive fallback;
- records every invocation.

The witness requires fail-closed launch, proves that exact-name identity inspection was reached, forbids container creation without admitted network identity, and requires that no network removal command be issued from the generated correlation after identity admission fails. It intentionally does not freeze the final public error taxonomy before the causal RED executes.

This is checked-in source RED only until exact-head CI executes the witness for the intended cause. Predecessor execution does not transfer.

## Authority and constraints

Podman documents that `podman network create` displays the newly added network **name**, not the network ID. `podman network inspect` exposes `.ID` separately. Podman also accepts either network name or ID for container `--network`; this runtime deliberately chooses the inspected full ID after admission so later lifecycle decisions do not depend on re-resolving a name.

Podman further documents that `podman network rm --force` removes containers that use the named network, stopping running containers when necessary. Ordinary non-force removal instead reports an in-use network as a failure. Network-level force therefore expands destructive authority and cannot be used as a substitute for proving exact ownership.

Repository authority adds two relevant invariants: generated public correlation data does not recreate runtime-only cleanup authority, and uncertain cleanup is a failure rather than permission to broaden deletion scope.

## Decision boundary

The causal GREEN, once the new RED executes, must preserve the following state distinction:

1. **Before exact network ID admission:** no destructive network authority exists. A malformed/unavailable identity inspection must fail closed without deleting a network by generated correlation name. The failure may leave an orphan requiring separately controlled reconciliation; it must not guess ownership.
2. **After exact network ID admission:** the admitted ID becomes the only network selector eligible for destructive cleanup. Every partial-launch cleanup path must retain that ID.
3. **After successful lease publication:** public `qsr-net-*` correlation remains consumer-visible evidence, while the admitted network ID is retained separately in non-serializable cleanup authority.
4. **Explicit termination:** remove the exact admitted network ID without network-level `--force`; foreign membership must make cleanup fail closed as `CleanupFailed` rather than delete the foreign container.

## Alternatives rejected

- **Delete the generated name with `--force` when inspection fails.** Rejected because the runtime has just failed to prove identity, and Podman documents that force removal can also stop and remove containers using that name.
- **Delete the generated name without `--force`.** Safer than force but still rejected as ownership authority: a name is re-resolvable and identity admission has failed. Avoiding collateral container deletion does not prove that the selected network is this invocation's resource.
- **Treat `network create` stdout as the immutable ID.** Rejected because Podman documents that successful create prints the network name while inspect exposes `.ID` separately.
- **Silently ignore malformed identity evidence and continue container creation by name.** Rejected because it converts unavailable security/lifecycle evidence into affirmative authority and reintroduces the original acquired-identity defect.
- **Retry inspection until a parseable ID appears.** Not selected as a correctness repair. Retry can hide identity replacement/race conditions and does not establish that a later object is the exact object created by this invocation.

## Acceptance sequence

After exact-head execution of `podman_application_service_network_identity_failure_cleanup_owner_red.rs` proves the intended RED:

1. apply the smallest production repair that removes correlation-name destructive fallback before ID admission;
2. rerun the identity-failure witness and existing acquired-ID chronology controls;
3. execute and repair the post-acquisition partial-cleanup RED so the admitted ID survives every later failure path;
4. prove exact-container exclusive effective attachment by `NetworkID` before port/readiness;
5. separate public generated correlation from private admitted network cleanup authority for successful leases;
6. execute explicit foreign-member termination with exact non-force ID cleanup;
7. only then complete missing mandatory isolation-evidence admission and full exact-head coverage/review/real-runtime/release gates.

An orphan produced by identity-admission failure is evidence of incomplete cleanup, not permission to broaden authority. Any later orphan-reconciliation capability requires its own bounded ownership proof and must not infer identity from a mutable correlation string alone.

## References

Podman. (2026). *podman-network-create — Create a Podman network*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-create.1.html

Podman. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Podman. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html

Podman. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html
