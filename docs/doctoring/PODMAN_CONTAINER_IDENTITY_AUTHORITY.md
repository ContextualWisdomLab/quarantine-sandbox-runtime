# Podman Container Identity Authority Traceability

## Decision

After a successful `podman create`, quarantine-sandbox-runtime treats the acquired container ID as the lifecycle authority for the created object. Generated `qsr-app-*` and `qsr-cmd-*` names remain correlation metadata only. They are not sufficient authority for later inspection, start, process evidence, logs, wait, kill, stop, or removal.

The backend must reject contradictory inspection evidence before promoting any security or lifecycle attestation. In particular, `podman container inspect <acquired-id>` must describe the same container identity; a returned `Id` that does not equal the acquired ID is malformed isolation evidence and fails closed. Cleanup continues against the acquired identity rather than any mutable name or contradictory inspected identity.

## Current implementation and evidence

Current owner lineage: PR #112, base PR #14 exact `aca827d7bd45f3df289456176730c781bd6d1164`.

The implementation parses the create receipt into a bounded backend identifier, invokes later lifecycle operations with that identifier, and validates `container inspect` identity before accepting isolation evidence. The command-runtime regression in `tests/podman_command_execution_post_create_ownership_red.rs` proves that contradictory inspect identity fails before `top`, `wait`, or `logs` evidence can be trusted and that cleanup targets only the acquired ID. `tests/podman_service_container_identity_mismatch_coverage.rs` applies the same fail-closed rule to application-service launch and requires container/network cleanup before later process, network, or port evidence is consumed.

Exact predecessor `ba41d77d9f2f8db44b21c895c8ea31ad0f3deb3d` executed those service identity assertions under CI `34617820478`: verify and hosted negative rootless/AppArmor were GREEN; coverage evidence was generated and failed only the repository-wide 100% admission. The current descendant continues to carry those regressions. Fake Podman fixtures are contract evidence only and are not positive proof of real runtime confinement.

## Primary-source basis

Podman documents `podman create` as creating the container without starting it and printing the resulting container ID to standard output. The same command provides `--cidfile` specifically to write the container ID to a file. This makes the create receipt an explicit backend identity artifact rather than a display-name convention.

Podman documents `podman container inspect` as accepting a container name or ID and returning low-level JSON whose `.ID`/`Id` field is the full container ID. quarantine-sandbox-runtime therefore checks that inspected identity against the already acquired create identity before using the remainder of the inspection document as security evidence.

Podman also permits renaming containers; its rename documentation states that the old name is freed and becomes available for reuse. Names are therefore mutable aliases and cannot safely serve as destructive post-create authority in a hostile-workload runtime. This is also consistent with Podman networking documentation, where names and short IDs can both appear as DNS aliases.

## Rejected alternatives

Using the generated name for post-create cleanup was rejected because name reuse or rebinding can redirect destructive operations to a different same-principal container. Trusting whichever `Id` appears in later inspect output was rejected because that would allow contradictory backend evidence to replace the create receipt instead of being treated as an integrity failure. Accepting a short ID was rejected for the same boundary: the owner already has the concrete acquired identifier and does not need to weaken it to an alias.

## Remaining acceptance boundary

This identity contract does not close the release gate. The same integrated exact head still requires real rootless positive-LSM evidence, issue #35 live cgroup-v2 and `/tmp` enforcement, issue #43 real over-lease exact-ID termination and leak-free cleanup, 100% owned-production coverage, independent review/security, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback evidence.

## References

Podman. (n.d.). *podman-create — Create a new container*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (n.d.). *podman-container-inspect — Display a container’s configuration*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman. (n.d.). *podman-rename — Rename an existing container*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/v6.1.1/markdown/podman-rename.1.html
