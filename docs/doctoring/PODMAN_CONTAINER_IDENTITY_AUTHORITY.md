# Podman Container Identity Authority Traceability

## Decision

After a successful `podman create`, quarantine-sandbox-runtime treats the acquired container ID as the lifecycle authority for the created object. Generated `qsr-app-*` and `qsr-cmd-*` names remain correlation metadata only. They are not sufficient authority for later inspection, start, process evidence, logs, wait, kill, stop, or removal.

The backend must reject contradictory inspection evidence before promoting any security or lifecycle attestation. In particular, `podman container inspect <acquired-id>` must describe the same container identity; a returned `Id` that does not equal the acquired ID is malformed isolation evidence and fails closed. Cleanup continues against the acquired identity rather than any mutable name or contradictory inspected identity.

## Current implementation and evidence

Command-runtime owner lineage is PR #112 above canonical command-runtime PR #14 exact `aca827d7bd45f3df289456176730c781bd6d1164`. Application-service container lifecycle authority is not owned by #112: canonical application-service owner PR #21 and issue #113 own successful-create receipt recovery, exact-ID cleanup, and the rule that a generated `qsr-app-*` name is never destructive fallback authority.

The command implementation parses successful-create stdout as independent evidence and reconciles it with a runtime-owned `--cidfile` receipt when present. A contradictory valid receipt fails closed and cleanup targets only the receipt ID. The command-runtime regression in `tests/podman_command_execution_post_create_ownership_red.rs` proves that contradictory inspect identity fails before `top`, `wait`, or `logs` evidence can be trusted and that cleanup targets only the acquired ID.

PR #112 currently also carries an application-service malformed-stdout compatibility delta inherited from an owner-repair attempt. That delta is not canonical application-service authority and must not be integrated as an independent application-service implementation. Before #112 can merge into #14, its application-service behavior must either be superseded by the ordinary dependency-safe adoption of #21/#113 or removed without discarding any command-runtime, chronology, gate, coverage, or traceability delta. This is a single-writer/Bounded Context repair finding, not grounds to close #112.

The application-service contract itself is proven on #21: runtime-owned create receipt evidence, strict 64-character lowercase hexadecimal identity admission, exact-ID lifecycle/cleanup, generated-name destructive-fallback rejection, forged-lease cleanup-authority rejection, and invocation-identity separation are hosted exact-head GREEN there. Those #21 checks do not transfer to #112 or #14 before ordinary integration.

Exact predecessor `ba41d77d9f2f8db44b21c895c8ea31ad0f3deb3d` executed service identity assertions under CI `34617820478`: verify and hosted negative rootless/AppArmor were GREEN; coverage evidence was generated and failed only the repository-wide 100% admission. That evidence remains causal history only; current authority for application-service receipt/lifecycle semantics is #21/#113.

## Primary-source basis

Podman documents `podman create` as creating the container without starting it and printing the resulting container ID to standard output. The same command provides `--cidfile` specifically to write the container ID to a file. This makes the create receipt an explicit backend identity artifact rather than a display-name convention.

Podman documents `podman container inspect` as accepting a container name or ID and returning low-level JSON whose `.ID`/`Id` field is the full container ID. quarantine-sandbox-runtime therefore checks that inspected identity against the already acquired create identity before using the remainder of the inspection document as security evidence.

Podman also permits renaming containers; its rename documentation states that the old name is freed and becomes available for reuse. Names are therefore mutable aliases and cannot safely serve as destructive post-create authority in a hostile-workload runtime. This is also consistent with Podman networking documentation, where names and short IDs can both appear as DNS aliases.

## Rejected alternatives

Using the generated name for post-create cleanup was rejected because name reuse or rebinding can redirect destructive operations to a different same-principal container. Trusting whichever `Id` appears in later inspect output was rejected because that would allow contradictory backend evidence to replace the create receipt instead of being treated as an integrity failure. Accepting a short ID was rejected for the same boundary: the owner already has the concrete acquired identifier and does not need to weaken it to an alias.

Copying #21 application-service source into #112 is also rejected. #21 is a mutable sibling owner branch and the repository contract requires dependency-safe ordinary integration of released/stabilized owner deltas rather than source duplication across bounded contexts.

## Remaining acceptance boundary

This identity contract does not close the release gate. The same integrated exact head still requires real rootless positive-LSM evidence, issue #35 live cgroup-v2 and `/tmp` enforcement, issue #43 real over-lease exact-ID termination and leak-free cleanup, 100% owned-production coverage, independent review/security, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback evidence.

PR #112 additionally requires the application-service owner-boundary finding above to be resolved before merge. PR #21 remains separately blocked on positive effective-LSM/runtime evidence, qualifying exact-head independent review/security, ordinary dependency-safe protected integration, and immutable release evidence.

## References

Podman. (n.d.). *podman-create — Create a new container*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman. (n.d.). *podman-container-inspect — Display a container’s configuration*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman. (n.d.). *podman-rename — Rename an existing container*. Podman documentation. Retrieved September 12, 2026, from https://docs.podman.io/en/v6.1.1/markdown/podman-rename.1.html
