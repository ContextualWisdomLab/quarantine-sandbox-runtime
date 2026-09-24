# Application-service cleanup authority traceability

## Problem and bounded-context boundary

`ApplicationServiceLease` is consumer-visible evidence. It may be serialized, persisted, copied, or reconstructed from untrusted input. It is therefore not a capability for Podman destruction.

At the original #42 RED, `RootlessPodmanAdapter::terminate_at` selected container and network cleanup targets directly from lease-visible identifiers. A caller that supplied lease-shaped JSON could consequently name same-principal resources that the runtime had never admitted. The application-service runtime must instead destroy only identities it acquired and retained while creating the resources.

This boundary is separate from, but composes with:

- #20: collision resistance of generated runtime correlation names;
- #40: lifecycle operations bound to the exact long container ID admitted from Podman;
- #41: retry convergence after partial cleanup without weakening foreign-member safety;
- #144: a network creation-event candidate is inspection authority only until P0 admission succeeds;
- #145: successful-lease termination must use the admitted exact network ID and must not use network-level `--force`;
- #146: public cleanup-receipt identifiers remain consumer-neutral correlations/evidence rather than backend selectors.

## Authority

- Joint Task Force. (2020, updated 2026). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5
- Joint Task Force. (2022, Release 5.2.0 updated 2025). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53Ar5
- Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
- Serde Project. (n.d.). *Using derive*. https://serde.rs/derive.html

## Executed evidence chain

| Evidence | Exact responsibility |
| --- | --- |
| #42 RED / `1d0cf2f47a8bd9df6594c734806a6c9c912fe0ed` | Proved that deserialized lease identifiers could recreate destructive cleanup selection. |
| Native CI `34299565806`, verify `102303404021` | Executed the forged `foreign-container` / `foreign-network` cleanup path. |
| `b0bcbe90ef034115ec266ffed5937aed1ee75150` | Introduced crate-private, non-serializable `ApplicationServiceCleanupAuthority`. |
| `e15980b820becd13c7e5756a3adca1f6504cd92c` | Required private authority in `terminate_at`; evidence-only leases fail closed as `CleanupAuthorityUnavailable`. |
| #145 / `2eea75b4f40faf3c95e7d3e8484525e51a845f25`, CI `35920385591` | Executed the successful-lease foreign-member RED: network-level `podman network rm --force` could delete a foreign member and still report successful cleanup. |
| #127 helper v1 `35948531229` | Removed network-level force and advanced the RED to the private-network-selector boundary; the run then stopped on stale broad fixture assertions. |
| #127 helper v2 `35967668419`, job `107529836056` | Completed the bounded repair on trigger exact `c08086f8fa72644a30fc9f062624a2a233bcb6d3`; focused termination, forged-lease, broad application-service tests and workspace check passed before the helper deleted itself and pushed the source commit. |
| #127 source `64956199a66164e31d58aaba436eda0d74349afe` | Carries exact container and admitted-network selectors in private cleanup authority, removes network-level force from explicit termination, updates broad fixtures, and contains no temporary source-fix workflow. |

The helper-generated source commit is a code delta, not repository-wide GREEN. Its automatically synchronized PR CI `35986337451` contains zero jobs and ended `action_required`; it therefore provides no test, coverage, lint, rustdoc, runtime, or security evidence for `64956199...`.

## Current selected contract

Successful launch now deliberately maintains two identity layers:

1. `RuntimeLeaseMetadata.sandbox_id` and `.network_id` remain generated `qsr-app-*` / `qsr-net-*` correlation values suitable for consumer evidence and audit joins.
2. `ApplicationServiceCleanupAuthority` retains the exact admitted Podman container ID and exact admitted Podman network ID. The type is crate-private and not reconstructed by Serde.

`ApplicationServiceLease::new_with_cleanup_resource_ids(...)` receives those backend selectors separately from public metadata. `terminate_at()` stops and force-removes only the exact container selector and removes the exact network selector with non-force `podman network rm <id>`. A deserialized lease has no private cleanup authority and cannot nominate either target.

Network-level `--force` is intentionally forbidden. Container-level `rm --force <exact-container-id>` remains a different primitive: it acts on the one admitted container rather than delegating deletion of foreign network members.

## Remaining authority gaps

This repair does not complete the network lifecycle.

First, current `acquire_network_id()` still treats the creation-event candidate as destructive authority before admission. If exact-ID inspection fails, parsing fails, the ID is malformed, or P0 state contradicts the expected name/internal/DNS policy, current code calls `cleanup_admitted_network(candidate)` before the candidate has actually been admitted. #144 already has execution-backed hostile evidence for this authority inversion. The minimum owner repair is candidate → exact inspection → full P0 corroboration → admitted ID; pre-admission orphan reconciliation belongs to #141 rather than immediate deletion.

Second, #41 remains required after exact successful-lease authority exists. A first termination may remove the admitted container and then fail non-force network removal because a foreign member is still attached. A retry must converge after that foreign member disappears even though the container is already absent. Any use of Podman `--ignore` must be limited to the exact admitted selector and already-absent state; it must not turn an in-use network or another backend failure into success.

Third, the Podman-v6 event transport witness remains separate. Upstream event JSON uses lowercase `network`; the dedicated checked-in witness must execute on canonical ancestry before changing the current `NetworkCreationEvent` serde mapping or broad fixture casing.

Finally, #146 owns public `CleanupReceipt` semantics. Receipt identifiers attest the logical correlation whose cleanup completed; they do not expose or recreate private backend selectors.

## Rejected alternatives

- accepting any schema-valid or well-shaped lease identifier as ownership proof;
- reconstructing private cleanup authority during deserialization;
- deriving destructive selectors from `qsr-app-*` or `qsr-net-*` correlations;
- restoring network-level `--force` to make foreign-member cleanup appear successful;
- deleting a creation-event candidate before the P0 admission contract succeeds;
- masking partial-cleanup retry failures by treating every Podman error as already absent.

## Evidence level and release gate

The forged-lease boundary and the bounded #145 repair have causal execution evidence, and helper v2 produced source exact `64956199...`. That exact is not integration or release GREEN because its normal PR CI did not execute any jobs, #144 remains unfixed on current source, #41 retry convergence is still open, positive effective-LSM evidence is absent, and immutable publication has not occurred.

Release acceptance still requires one unchanged dependency-safe protected integrated exact with repository policy, rustfmt, full locked workspace/all-target tests, Clippy and public/private rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security/thread gates, real rootless enforcement plus positive effective-LSM evidence, version/CHANGELOG, immutable package/tag/Release authority, SBOM/provenance/reproducibility, and rollback evidence.
