# Application-service cleanup authority traceability

## Problem and boundary

At the executed RED state, `ApplicationServiceLease` was a public serializable/deserializable evidence contract and `RootlessPodmanAdapter::terminate_at` used `sandbox_id` and `network_id` from that value directly as Podman destruction targets. That conflated consumer-visible evidence with runtime-owned lifecycle authority.

The runtime owns cleanup of resources it created. It does not gain authority to stop or remove an arbitrary same-principal container or network merely because a caller can present a lease-shaped JSON object naming that resource. Upstream consumers retain application authorization; the infrastructure adapter must still enforce its own resource-ownership boundary.

Issue #42 owns this defect. It is distinct from:

- #20: collision resistance of generated application-service runtime names;
- #40: binding post-create lifecycle operations to the exact long ID returned by successful `podman create`;
- #41: avoiding network-level `--force` because that delegates deletion of foreign network members to Podman.

## Authority

- Joint Task Force. (2020, updated 2026). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. AC-3 requires access enforcement and AC-6 requires least privilege for users and processes acting on their behalf. https://doi.org/10.6028/NIST.SP.800-53r5
- Joint Task Force. (2022, Release 5.2.0 updated 2025). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A Rev. 5). National Institute of Standards and Technology. The AC-6 assessment procedures include testing mechanisms that implement least-privilege restrictions. https://doi.org/10.6028/NIST.SP.800-53Ar5
- Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
- Serde Project. (n.d.). *Using derive*. Deriving `Deserialize` implements construction of the Rust data structure from serialized input. https://serde.rs/derive.html

## Code and test chain

| Evidence | Exact responsibility |
| --- | --- |
| `src/application_service/mod.rs::ApplicationServiceLease` at RED head `1d0cf2f47a8bd9df6594c734806a6c9c912fe0ed` | Consumer-visible lease/evidence contract whose deserialized identifiers could recreate cleanup selection. |
| `tests/podman_application_service_forged_lease_ownership_red.rs` | Hostile RED proving that a lease created only from caller JSON must not select Podman stop/remove/network-remove targets. |
| Native CI `34299565806`, verify `102303404021` | Causal execution showing forged `foreign-container` / `foreign-network` values reached destructive cleanup. |
| `b0bcbe90ef034115ec266ffed5937aed1ee75150` | Captures crate-private, non-serializable `ApplicationServiceCleanupAuthority` only during runtime construction. |
| `e15980b820becd13c7e5756a3adca1f6504cd92c` | Makes `terminate_at` require `lease.cleanup_authority()` and fail closed as `CleanupAuthorityUnavailable` before destructive Podman commands when authority is absent. |
| Exact later ancestry `368e20eeeb0ac3d573a913af981dcb5dd4104b1a` | Regression-verified the forged-lease authority boundary before execution advanced to the independent #20 collision RED. |
| Issue #42 / Draft #21 | Decision, integration, and completion authority for separating evidence from cleanup capability. |

The RED scenario is intentionally small: no launch occurs, no runtime-owned resource authority exists, and fake Podman reports successful destruction if the forged identifiers are used. The RED head failed because deserialized evidence was sufficient to select those resources. Current #21 production no longer behaves that way: runtime construction captures private cleanup authority, while a deserialized lease has none and `terminate_at` returns `CleanupAuthorityUnavailable` before issuing Podman destruction.

## Selected causal repair

`ApplicationServiceLease` remains consumer-visible evidence/correlation, but destructive lifecycle selection is now gated by crate-private runtime-owned provenance that Serde does not reconstruct. This is deliberately stronger than identifier-shape validation: `request_id`, `sandbox_id`, `network_id`, policy metadata, endpoint fields, and attestation booleans remain replayable evidence once serialized and cannot independently authorize backend destruction.

The private cleanup authority currently retains the runtime-selected container/network targets and shutdown grace. #40 remains an independent prerequisite because the application-service Podman adapter still must replace the generated post-create container selector with the exact admitted long ID returned by successful `podman create`; that acquired ID must then flow into private cleanup authority without changing the public generated-name correlation field.

Rejected alternatives remain:

- accepting any schema-valid or well-shaped lease identifiers as ownership proof;
- reconstructing private authority during deserialization;
- restoring generated-name destructive fallback when private authority is absent;
- weakening #40 by treating collision-resistant generated names as equivalent to acquired backend identity.

## Evidence levels

The executed RED plus later regression establishes that public/deserialized lease evidence is no longer accepted as destructive authority on this Draft lineage. It is not protected-integrated or release GREEN. Release acceptance still requires a legitimate launch → readiness → termination path using exact runtime-acquired identity, #40 lifecycle selector repair after its own causal RED, failure-path cleanup precedence, no foreign-resource effects, real rootless Podman execution, positive effective LSM evidence, full owned coverage/rustdoc/security/review, and exact protected integration evidence.
