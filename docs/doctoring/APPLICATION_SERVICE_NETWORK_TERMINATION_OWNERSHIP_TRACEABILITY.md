# Application-service explicit-termination network authority

## Problem

The application-service Podman adapter has two network cleanup paths: partial-launch cleanup and explicit `terminate_at`. The existing foreign-member regression covers only the partial-launch path. On the current parent `ccfbcd4ed4e70f95b547b4fb3bc76a8276ca0efd`, explicit termination still invokes `podman network rm --force <lease.network_id>`, so a caller-visible correlation identifier can become a destructive selector after a service has already been published.

Podman documents `network rm --force` as removing all containers that use the named network, including stopping and removing a running container. Without `--force`, an in-use network is reported as an error with exit status 2. Network inspection also exposes a distinct network `.ID` and the containers attached to a network. Those facts make force-removal inappropriate for a runtime that owns one sandbox container but does not own arbitrary same-principal containers that may later attach to the network.

## Test-first authority

Test commit `5d7bc3f1bd7783d05961c64224bc6d426b833aca` adds `tests/podman_application_service_network_termination_ownership_red.rs` without changing production code.

The witness first launches a valid service. Only after the lease exists does it create a foreign-member marker, then calls the public `RootlessPodmanAdapter::terminate_at` path. The fake Podman process models the documented semantics:

- `network rm --force` removes the foreign member and returns success;
- non-force `network rm` returns exit status 2 while the foreign member remains;
- container stop/removal succeeds, isolating network cleanup as the failure cause.

Acceptance requires `ApplicationServiceError::CleanupFailed`, preservation of the foreign marker, and no network-level `--force` invocation. This is intentionally a RED-only descendant of #119. The test has not been promoted to executed causal RED while GitHub-hosted jobs remain unassigned.

## Selected repair direction

The later production GREEN belongs to the same canonical network-lifecycle owner as #23/#41/#48 and must compose, rather than duplicate, those contracts:

1. acquire the created Podman network `.ID` before container creation;
2. bind the container to the acquired network identity rather than a re-resolvable generated name;
3. verify the effective attachment set before readiness publication;
4. keep public `qsr-net-*` as correlation metadata unless a versioned public contract explicitly changes it;
5. keep destructive network authority private and exact;
6. remove the owned network without network-level force deletion;
7. if foreign attachment prevents network deletion, return `CleanupFailed` while preserving the foreign resource.

Partial-launch and explicit-termination witnesses remain separate because they exercise different lifecycle states and error-precedence paths.

## References

Podman Project. (n.d.). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. Retrieved September 15, 2026, from https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Podman Project. (n.d.). *podman-network-rm — Remove one or more networks*. Podman documentation. Retrieved September 15, 2026, from https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html
