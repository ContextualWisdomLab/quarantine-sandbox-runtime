# Command create-failure cleanup ownership traceability

Status: Draft evidence record for issue #33 and Draft PR #14.

## Problem

`podman create --name <requested-name>` can fail after the runtime has done enough work that a container may exist. A requested name is correlation metadata, not proof that this invocation owns whichever container currently resolves under that name. The previous command-execution failure path called `podman rm --force --ignore <requested-name>` after any failed create. The exact-head RED on predecessor `9268842a9ab741448a43979f9252d10838aa5f60`, CI run `34013162142`, verify job `101432344588`, demonstrated the unsafe case: a pre-existing same-name foreign marker was removed when create failed.

This is a cleanup-authorization defect in the `sandbox_execution` Core boundary, surfaced through the Podman infrastructure adapter. It is not solved by lowering collision probability, generating a longer name, or assuming a failed create is atomic.

## Constraints and invariants

- Destructive cleanup needs invocation-owned resource identity, not a requested name.
- A create failure with no trustworthy ownership receipt must not authorize destructive name-based cleanup.
- If an exact owned resource identity is available, cleanup is attempted and cleanup failure takes precedence over the earlier create failure because leak risk remains unresolved.
- The repair must not pre-empt issue #36, which separately owns post-success lifecycle migration from mutable name addressing to the exact acquired long container ID.
- Runtime-owned temporary receipt material must be private to the runtime process and removed automatically after the call.
- Malformed or unreadable ownership evidence fails closed; it is never interpreted as a container name.

## Alternatives considered

1. Keep `rm --force --ignore <requested-name>` after failed create. Rejected because name equality does not prove ownership and the causal RED shows destructive cross-invocation behavior.
2. Use a longer or random runtime name and retain name cleanup. Rejected because probability reduction is not an authorization proof.
3. List or inspect by label/name after failure and infer ownership. Rejected for the minimal repair because post-hoc lookup is race-prone and weaker than an invocation-bound creation receipt.
4. Request Podman's `--cidfile=<file>` and treat only the exact returned container ID as cleanup authority. Selected. Podman documents that `--cidfile` writes the created container ID to a file; this creates an invocation-scoped handoff independent of name lookup.

## RED and repair chain

- Causal RED authority: predecessor `9268842a9ab741448a43979f9252d10838aa5f60`, native CI `34013162142`, verify job `101432344588`, `failed_create_without_owned_container_id_does_not_delete_same_name_resource`.
- Test repair `701fe7fdfc2829cb82ea7b8021a95e916d065181`: the older partial-create cleanup fixture now writes a 64-hex owned ID to the runtime-supplied cidfile and requires cleanup by that exact ID. This removes the previous false-GREEN assumption that the requested name was ownership evidence.
- Minimal production candidate `d45801bbba44a536f586ed611fae60c3f03eb2ac`: allocates a private temporary cidfile, passes `--cidfile=<path>` to `podman create`, performs no destructive cleanup when failed create produces no receipt, and cleans by the exact 64-hex ID when a valid receipt exists. Malformed/read failures fail closed. Post-success name-based lifecycle remains unchanged for issue #36.

The candidate is not accepted as GREEN until fresh exact-head CI executes the foreign-marker RED, owned-partial-create cleanup cases, verification, statement/branch coverage and applicable real-runtime gates on the unchanged head.

## Security and operational effect

The repair changes cleanup authorization, not container isolation policy. It prevents a failed invocation from deleting an unrelated same-name container while retaining fail-closed leak reporting for an invocation-owned partial create. It also provides a concrete ownership handoff that issue #36 can later extend across the complete successful lifecycle without changing the issue #33 invariant.

Remaining risks are intentionally visible: receipt-file creation/read/parsing branches need exact coverage; a successful create still uses the requested name in later operations until #36 crosses its own RED gate; and positive rootless/LSM runtime evidence remains a separate release gate.

## References

Podman. (2026). *podman-create — Podman documentation*. https://docs.podman.io/en/stable/markdown/podman-create.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
