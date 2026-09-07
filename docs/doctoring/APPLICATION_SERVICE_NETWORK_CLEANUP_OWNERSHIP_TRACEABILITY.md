# Application-service network cleanup ownership traceability

## Problem

The application-service adapter currently calls `podman network rm --force <qsr-net-*>` during partial-launch cleanup and explicit lease termination. The sandbox runtime owns the sandbox container and the network it created; it does not own arbitrary same-principal containers that may have attached to that network after creation.

Podman documents a materially broader meaning for `network rm --force`: all containers using the named network are removed, with running containers stopped and removed as part of the network operation. Using that primitive therefore converts network cleanup into recursive container deletion outside the sandbox aggregate's ownership boundary.

## Constraint and alternatives

The cleanup contract must prefer a visible cleanup failure over collateral deletion. Three approaches were considered:

- keep `network rm --force` because the network name is runtime-generated — rejected because generated network ownership does not confer ownership of every container attached to it;
- enumerate attached containers and force-remove those that look runtime-owned — rejected for the current increment because this recreates container-ownership policy inside network cleanup and remains vulnerable to incomplete/stale enumeration;
- remove only the invocation-owned container through its ownership-bound lifecycle identity, then remove the network without force — selected. If another container still holds the network, Podman must report the network as in use and the runtime must surface `CleanupFailed` without touching that container.

An already-absent network may later use Podman's idempotent `--ignore` semantics if the absence is treated as acceptable cleanup evidence. `--ignore` must never be combined with authority to remove attached foreign containers.

## Evidence chain

| Authority / evidence | Runtime meaning | Repository surface |
| --- | --- | --- |
| Souppaya, Morello, & Scarfone (2017), NIST SP 800-190 | Container runtime isolation and lifecycle controls must not enlarge one workload's authority over others. | Issue #41; this record |
| Podman Authors (2026), `podman-network-rm` | `--force` removes containers using the network, including running containers. | `RootlessPodmanAdapter::cleanup_network`; `RootlessPodmanAdapter::terminate_at` |
| Podman Authors (2026), `podman-network-inspect` | Network state exposes attached containers as evidence; attachment is not authorization to delete them. | #22/#23 network-attestation boundary |
| RED commit `e8bee063dfcbd8d77e04b00b6cd23dc5c11fa5a5` | A failed sandbox container create leaves a foreign network member in place only if cleanup does not invoke network-level force removal. | `tests/podman_application_service_network_cleanup_ownership_red.rs` |

## RED and expected causal failure

The fake Podman creates the intended network, models a foreign container attached to it, and then fails sandbox container creation. Current production invokes `network rm --force`; the fixture records that as a foreign-container deletion side effect and reports network cleanup success. The test instead requires the foreign marker to remain unchanged, forbids `network rm --force`, and requires the non-force in-use condition to surface as `ApplicationServiceError::CleanupFailed`.

This is intentionally independent of #22/#23's sandbox-container attachment proof and #40's acquired container-ID lifecycle authority. A sandbox may be attached to exactly the right network and its own container may be addressed by the right ID while network-level force deletion is still over-broad.

## GREEN scope after causal RED executes

The smallest production repair is to remove network-level `--force` from partial-launch and explicit-termination cleanup. Owned container teardown remains a separate step and must eventually use #40's acquired backend identity. The network removal then succeeds only when no other member remains; an in-use network is a cleanup failure, not permission to delete that member.

Release evidence must later include a real rootless Podman case with an independently created foreign container attached to the sandbox network, proving that cleanup never stops or removes the foreign container and that cleanup uncertainty is reported rather than hidden.

## References

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
