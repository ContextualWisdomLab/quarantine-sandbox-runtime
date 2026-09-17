# Application-service effective-isolation gap-owner repair

## Scope

PR #9 owns effective isolation attestation before an application-service lease becomes publishable. Its domain truth is the application-service/Podman boundary that proves applied and effective rootless, privilege, namespace, seccomp/capability, LSM, network-publication and cleanup state before returning a lease. It does not own the repository-wide Gap ledger, caller-scoped lease ownership (#6), resource-enforcement expansion (#19), runtime invocation/lifecycle identity (#20/#40), or exact network-attachment work (#23).

## Retained causal evidence

The earlier exact `3fa5c5493fcbfbfb1c28b075e3bad30c03ea29b3` causal RED proved that the old runtime could return an application-service lease without effective sandbox inspection. The production lineage subsequently added effective inspection and fail-closed behavior.

Exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d` executed on GitHub-hosted Ubuntu 24.04 with rootless Podman 4.9.3, reached effective LSM verification, failed closed with `IsolationVerificationFailed { control_name: "lsm" }`, and rejected leaks. This is valid negative-LSM evidence, not positive confinement proof. Positive release evidence remains a separate `[self-hosted, linux, cwl-hostile-workload, selinux]` requirement.

Historical #9 exact `fa1a0faf83656777e2ac34d3f4a641f7b5c76543` preserved the effective-isolation production/tests while adopting then-current parent/root authority. Those predecessor observations do not transfer after parent movement.

## Single-writer and parent-adoption decision

Review `5230230124` found two related repair obligations: #9 still carried `docs/product-technical-gap-baseline.md`, although #121 is the repository-wide Gap owner, and parent #6 moved to `ecdd84836d1d04660f620156f2190d8eb5664837` during its own migration-first owner repair.

The repair therefore preserves #9-specific effective-isolation history in this owner-local record, then adopts the parent #6 owner-repair deltas by an ordinary non-force two-parent commit. The child must carry #6's `APPLICATION_SERVICE_LEASE_OWNERSHIP_GAP_OWNER_REPAIR.md` and the parent's restored global-Gap blob while retaining #9 production/tests unchanged.

No force push, destructive rebase, production weakening, path exclusion, or predecessor-evidence transfer is permitted. The resulting exact #9 head must reacquire its own full verification, complete coverage, review/security, hosted negative-LSM and applicable positive-LSM evidence before merge authority can advance.
