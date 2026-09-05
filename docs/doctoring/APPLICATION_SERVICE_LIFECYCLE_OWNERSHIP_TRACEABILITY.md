# Application-service lifecycle ownership traceability

## Decision under test

Application-service container lifecycle authority must remain bound to the exact long container ID returned by the successful `podman create` invocation. The generated `qsr-app-*` name remains correlation/audit metadata and may continue to back the public `sandbox_id` contract, but it is not sufficient destructive authority once an immutable backend ID has been acquired.

This distinction is required because the current adapter creates the container, parses the returned ID, and then re-resolves the generated name for `start`, `container inspect`, `top`, `port`, partial-launch cleanup, readiness-failure cleanup, and later lease termination. The existing `inspect.Id == acquired_id` comparison is useful defense in depth but occurs after the name-based lookup has already selected a target.

## Authority chain

| Evidence | What it establishes | Runtime consequence |
| --- | --- | --- |
| Podman `create` success stdout | Invocation acquired one concrete container ID. | Retain this ID as backend lifecycle ownership evidence. |
| Podman identifier semantics | Containers may be addressed by long ID, short ID, or name; names are human-friendly identifiers. | Prefer the exact acquired long ID for post-create lifecycle/destructive calls. |
| Podman container inspect | Inspect accepts name or ID and returns `Id`. | Address inspect by acquired ID and still require returned `Id` equality. |
| NIST SP 800-190 | The runtime is responsible for establishing and maintaining container isolation and lifecycle controls. | Cleanup and control operations must not cross an invocation ownership boundary. |
| Issue #40 / Draft #21 RED | Models immediate name rebinding to a foreign container after successful create. | Any post-create use of the generated name is a security failure even if names are collision-resistant. |

## RED evidence

`tests/podman_application_service_post_create_ownership_red.rs` creates an otherwise-positive fake-Podman application-service launch. The fake backend returns one fixed long owned ID from `create`, then treats every lifecycle operation addressed by the generated name as a foreign-resource side effect. The same operations addressed by the exact ID remain valid.

The test requires:

- launch succeeds without touching the foreign marker;
- public `sandbox_id` continues to look like `qsr-app-*`, so the repair does not silently redefine the existing consumer-facing identifier;
- `start`, `container inspect`, `top`, `port`, `stop`, and `rm` all target the acquired long ID;
- `terminate_at` preserves the same ownership binding;
- the runtime-owned network lifecycle remains separate and unchanged.

Current production is expected to RED at the first post-create name-based operation. Production must not change until that causal failure executes on an exact runner-backed head.

## Smallest GREEN boundary

Retain the acquired container ID in runtime lease metadata as a backend resource identity distinct from the generated correlation name. Thread it through post-create launch verification, failure cleanup, readiness cleanup, and explicit termination. Preserve the generated name for current audit/correlation and public contract semantics unless a separate versioned contract deliberately changes them.

Do not treat larger random names as a substitute for ID-bound lifecycle ownership. Do not weaken effective isolation, network binding, resource attestation, LSM proof, cleanup precedence, or caller-scoped idempotency.

## References

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman Authors. (2026). *podman-run — Run a command in a new container*. https://docs.podman.io/en/stable/markdown/podman-run.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
