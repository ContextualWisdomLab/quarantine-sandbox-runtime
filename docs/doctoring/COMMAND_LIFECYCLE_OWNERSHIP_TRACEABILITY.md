# Command Lifecycle Ownership Traceability

## Decision scope

This record owns issue #36: after a successful command-container create, the runtime must treat the acquired long Podman container ID as lifecycle authority. The generated `qsr-cmd-*` name remains correlation/audit metadata and continues to populate the existing result `sandbox_id`; it is not used to re-resolve or destroy the resource once creation has returned an ID.

This is an infrastructure ownership invariant inside the application-service supporting context. It is distinct from issue #33, which covers create failure before a trustworthy owned ID exists, and from issue #28, which reduces generated-name collisions but cannot turn a mutable name into ownership proof.

## Causal RED

On exact predecessor `bd5382d2df32d4f006f72314a8257417f6ac18e8`, native CI `34280048795`, branch-coverage job `102242369891` compiled and passed the preceding command-runtime regressions, including #32 mount-set, #39 explicit-host namespace, and both #34 invalid-UTF-8 tests. It then failed exactly at `tests/podman_command_execution_post_create_ownership_red.rs::successful_create_binds_every_lifecycle_operation_to_the_acquired_container_id` with `Backend(CleanupFailed)`.

The fake Podman returns invocation-owned ID `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa` from create. Every later operation addressed by that exact ID operates on the owned marker. Any operation addressed by the generated name marks a foreign same-principal resource as touched and fails. Current production therefore demonstrated the intended name-rebinding/TOCTOU ownership defect rather than a checkout, formatting, repository-policy, or unrelated fixture prerequisite.

The same predecessor's verify job stopped earlier at rustfmt on the newly added #34 strict-UTF-8 lines. Branch coverage nevertheless executed the compiled semantic suite and supplies the causal #36 RED; the formatting prerequisite is repaired in the same minimal source movement described below and must be re-proven on the final exact head.

## Minimum causal repair

Source commit `85b667b0bd3d5ea8bd7f0593ae91c2df12623579` makes the smallest post-create authority change:

- `start` uses the acquired long container ID;
- command `container inspect` and `top` use that ID and retain the defense-in-depth `inspect.Id == acquired_id` check;
- initial `wait`, timeout `kill`, post-kill `wait`, and `logs` use that ID;
- all post-create failure cleanup and successful final removal use `cleanup_owned_command_container*` with the ID;
- the generated name is still passed to create and remains `CommandExecutionResult.sandbox_id`, preserving released result semantics;
- the create-failure/no-owned-ID path remains separate under issue #33;
- the same source commit applies rustfmt's required layout to #34 strict UTF-8 conversion without changing that behavior.

No force push or destructive rebase is involved.

## Alternatives rejected

- Keep names after create because generated names are 128-bit collision resistant: rejected because collision probability does not remove mutable-name resolution or rebinding as an authorization problem.
- Use a short container ID: rejected because the exact long ID is already available and avoids unnecessary ambiguity.
- Change public `sandbox_id` from the correlation name to the Podman ID in-place: rejected because issue #36 requires lifecycle authority, not an unversioned public-contract semantic change.
- Rely only on `inspect.Id == acquired_id` while other operations continue by name: rejected because `start`, `wait`, `logs`, `kill`, and destructive removal would remain independently vulnerable to name rebinding.
- Let cleanup revert to the generated name after a later error: rejected because cleanup is destructive authority and must remain bound to the same acquired resource identity.

## Security and buyer effect

The repaired path binds the full successful-create lifecycle to one immutable backend identity. A sibling invocation or same-principal actor cannot redirect a later start/attest/wait/log/kill/remove operation merely by changing what the human-readable name resolves to. Result/audit correlation remains stable, while destructive authority becomes explicit and reproducible.

This does not close issue #33's pre-ownership create-failure cases, issue #25's pre-payload attestation problem, or real rootless-Podman acceptance. Those remain separate release gates.

## Release gates

Before #36 can be considered GREEN for release, one unchanged exact head must pass the dedicated name-rebinding regression, #33 cleanup-ownership regressions, full Rust tests, rustfmt, Clippy/rustdoc, complete owned-production statement/function/region/branch coverage, repository policy, qualifying review, hosted negative confinement, dedicated positive effective-LSM, central security/dependency gates, and applicable real rootless-Podman E2E. Predecessor passes do not transfer to a moved head.

## References

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
