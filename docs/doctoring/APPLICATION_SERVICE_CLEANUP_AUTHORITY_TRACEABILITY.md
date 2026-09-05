# Application-service cleanup authority traceability

## Problem and boundary

`ApplicationServiceLease` is a public serializable evidence contract. It currently also derives `Deserialize`, while `RootlessPodmanAdapter::terminate_at` uses `sandbox_id` and `network_id` from that value as direct Podman destruction targets. That conflates consumer-visible evidence with runtime-owned lifecycle authority.

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
| `src/application_service/mod.rs::ApplicationServiceLease` | Consumer-visible lease/evidence contract; currently derives `Deserialize`. |
| `src/infrastructure/podman.rs::RootlessPodmanAdapter::terminate_at` | Destructive backend lifecycle boundary; currently trusts lease resource identifiers. |
| `tests/podman_application_service_forged_lease_ownership_red.rs` | Hostile RED proving that a lease created only from caller JSON must not select Podman stop/remove/network-remove targets. |
| Issue #42 | Decision and completion authority for separating evidence from cleanup capability. |

The RED keeps the scenario intentionally small: no launch occurs, no runtime-owned resource receipt exists, and fake Podman reports successful destruction if the forged identifiers are used. Current production should therefore fail because `terminate_at` accepts the deserialized value as sufficient authority.

## Smallest causal repair after executed RED

Preserve `ApplicationServiceLease` as evidence/correlation where the public contract needs it, but move destructive lifecycle selection behind runtime-owned provenance that cannot be manufactured by deserializing the lease. The preferred model is a non-serializable internal cleanup handle or equivalent authenticated runtime authority record bound to the acquired container ID and network ownership receipt.

Identifier-shape validation is not ownership proof. `request_id`, `sandbox_id`, `network_id`, policy metadata, endpoint fields, and attestation booleans remain caller-replayable once serialized and cannot independently authorize backend destruction.

## Evidence levels

A GREEN unit regression establishes only that the public/deserialized lease is no longer accepted as destructive authority. Release acceptance still requires a legitimate launch → readiness → termination path using exact runtime-acquired identity, failure-path cleanup precedence, no foreign-resource effects, real rootless Podman execution, positive effective LSM evidence, full owned coverage/rustdoc/security/review, and exact protected integration evidence.
