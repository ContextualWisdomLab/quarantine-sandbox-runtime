# Command malformed-create receipt ownership traceability

Last reviewed: 2026-09-11 KST

## Authority and scope

This record belongs to the `sandbox_execution` Core bounded context and its `infrastructure::podman` adapter. Issue #105 and Draft #106 supplied the historical causal RED and first minimum repair; canonical Draft #14 now owns the integrated command-runtime behavior and its successor regressions.

The boundary is lifecycle and destructive authority after `podman create` has created a resource but stdout cannot be admitted as the concrete container identity. It does not change `application_service`, `artifact_analysis`, consumer authorization, public wire schemas, or provider/model semantics.

## Problem

The command runtime provisions a private runtime-owned `--cidfile=<path>` for create. Before issue #105's repair, the successful-create malformed-stdout path could fall back to the generated `qsr-cmd-*` correlation name for cleanup even when the cidfile already contained the concrete created-container ID.

That fallback violated the lifecycle-ownership invariant already established for the normal post-create path: once a concrete resource identity is available, destructive operations must be selected by that exact identity rather than by re-resolving a mutable name. The malformed-success path is especially important because stdout and a runtime-owned cidfile are independent observation channels for the same creation event.

Podman documents both behaviors: `podman create` prints the created container ID and `--cidfile=file` writes the container ID to a caller-selected file. The runtime therefore treats an admitted value from its own receipt location as concrete lifecycle evidence while retaining malformed stdout as an evidence failure. NIST SP 800-190 is broader lifecycle authority: it supports fail-closed, auditable management of container resources, not a particular Podman CLI parsing rule.

## Historical causal RED

Draft #106 test-bearing commit `f6b68d810df44780b273289d1d77590440a4937c` introduced `tests/podman_command_execution_malformed_create_receipt_ownership_red.rs` without production changes. Its fake Podman created a container, wrote a valid 64-character lowercase hexadecimal ID to the runtime-owned cidfile, emitted unusable stdout, allowed cleanup by the acquired ID, and rejected cleanup by the generated correlation name.

Exact RED head `054a0e8fc6c746398295703dfd71a138efe404c5`, native CI `34284356731`, branch-coverage job `102256345545`, reached the focused regression and returned `CleanupFailed` where the contract required the original `MalformedIsolationInspection { operation: "container_create" }` after successful acquired-ID cleanup. That execution isolated the intended ownership defect. Broad lanes on the same historical ancestry encountered separately tracked process-boundary failures and were not used as substitute evidence.

## Minimum repair and canonical adoption

Historical production commit `4f7a670fcec0663ef13a04c6e2ea42e4505df86a` repaired the successful-create malformed-stdout branch. Canonical #14 retains the same security semantics in the current command path:

1. preserve the original typed create-evidence error;
2. read the existing runtime-owned cidfile through `read_command_create_receipt`;
3. admit only an exact 64-character lowercase hexadecimal container ID;
4. when an acquired ID exists, run cleanup only against that ID and preserve cleanup-failure precedence;
5. when the receipt is absent, return the original malformed-create error without destructive generated-name lookup;
6. when the receipt itself is unreadable or malformed, fail closed on receipt evidence;
7. after a normal successful identity acquisition, address init, inspection, start, wait, termination, logs, and cleanup by the acquired ID rather than `qsr-cmd-*`.

The current canonical implementation uses `--cidfile` on create and calls `read_command_create_receipt` on failed create and malformed successful stdout. `qsr-cmd-*` remains correlation/result metadata rather than destructive authority.

## Regression evolution

The first #106 fixture emitted `not-a-container-id`. Later review showed that this spelling is still syntactically admissible by the backend identifier grammar, so it did not reliably force the malformed-stdout branch. Canonical #14 deliberately uses `not a container id`, containing spaces, as the malformed witness.

The current acquired-receipt regression calls the legacy command composition test seam to isolate this ownership branch from the newer runtime-gate lifecycle. It requires the original malformed-create error, proves `--cidfile=` was supplied, requires exactly one `rm --force --ignore <acquired-id>`, and proves the removal target contains no generated `qsr-cmd-*` name.

The complementary no-receipt regression models successful create with malformed/empty stdout and no cidfile. It requires the same original malformed-create error and proves that no `rm --force` call occurs at all. Together these cases distinguish three states that must not be collapsed: valid acquired identity, absent identity evidence, and malformed/unreadable identity evidence.

## Alternatives considered

### Generated-name cleanup after malformed stdout — rejected

A generated name is correlation metadata. Re-resolving it after creation can select a different same-principal resource and reopens the name-rebinding boundary that exact-ID lifecycle ownership was designed to close.

### Normalize or permissively parse malformed stdout — rejected

Malformed output is evidence failure. Truncation, short-ID admission, case normalization, or other permissive parsing would widen destructive authority and hide a backend-contract violation.

### Retry create or infer ownership from request intent — rejected

A retry creates another lifecycle event, while request intent says nothing about which concrete resource actually exists. The already-provisioned runtime-owned cidfile is the bounded evidence source.

### Treat receipt absence as permission to clean by name — rejected

Without an admitted concrete identity the runtime cannot prove the target selected by a destructive name lookup. Absence therefore preserves the original error and authorizes no name-based removal.

## Current verification and release gate

Canonical #14 predecessor `3271a2694b57c4c3cc7c64d4d94f666b06e4cf0f` had full hosted verify GREEN and the current ownership regressions in its workspace suite. Test-only descendant `cd619aa648af03c7c40e0566a09d1c330f20108e` also completed verify GREEN before this documentation adoption. Those executions validate the inherited behavior but do not transfer exact-head release authority after this documentation commit.

The current canonical head must therefore reacquire exact checkout, repository validation, rustfmt, full workspace tests, Clippy, rustdoc, complete owned-production line/function/region/branch coverage, hosted negative confinement, qualifying review/security gates, and dedicated positive effective-LSM evidence. Real rootless-runtime evidence remains necessary for claims about effective isolation. Protected integration must precede immutable version/package/tag/release, SBOM, provenance, reproducibility, rollback evidence, and any consumer version bump.

## References

Podman. (n.d.). *podman-create — Podman documentation*. Retrieved September 11, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
