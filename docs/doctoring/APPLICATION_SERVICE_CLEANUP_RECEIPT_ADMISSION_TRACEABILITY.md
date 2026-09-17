# Application-service cleanup receipt admission TRACEABILITY

Date: 2026-09-08
Status: Causal RED → minimum production candidate; exact candidate GREEN pending
Parent authority: root PR #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`
Owner: issue #87 / PR #88

## Problem

`CleanupReceipt` is a public runtime-evidence type and historically derived `Deserialize` directly, while the invariant-establishing constructor is crate-owned `CleanupReceipt::complete`. Direct Serde construction therefore bypassed the invariants already published by `schemas/application-service-cleanup.schema.json`.

The published `application-service-cleanup-1.0.0` document is closed (`additionalProperties: false`), fixes `schema_version` to `1.0.0`, constrains sandbox/network identifiers to `^[a-z0-9-]{1,64}$`, and fixes both removal fields to `true`. Private Rust fields are not an admission control when the containing public type implements `Deserialize`.

This is evidence integrity, not cleanup authority. Issue #42 separately owns whether caller-visible state can select destructive backend targets. Issue #84 / Draft #85 owns forged lease and isolation-attestation admission. Issue #87 owns only whether serialized cleanup evidence can become the same Rust `CleanupReceipt` type without satisfying the existing cleanup schema.

## Causal RED

Dependency-safe RED head `c653a9d8ad4418b8cb217b38f9bd8e534d8528a0` adopted root exact `5c6a44bb...` by ordinary two-parent, non-force integration while preserving only the cleanup-receipt test/TRACEABILITY delta.

Native CI `34202793075` executed that exact head. Verify `101985233773` passed exact checkout, repository policy, coverage-parser checks, and `cargo fmt --check`, then failed at the Rust test stage. Coverage `101985233664` and branch-coverage `101985233441` likewise failed while generating Rust coverage evidence. Hosted negative rootless/AppArmor `101985233818` was GREEN; positive-LSM `101985233755` remained a separate queued release/security lane. This establishes a causal RED at the public cleanup-receipt admission boundary rather than a formatting-only failure.

`tests/application_service_cleanup_deserialization_red.rs` preserves one valid wire value and requires deserialization failure for:

- unsupported cleanup schema version;
- either removal claim set to `false`;
- empty, oversized, uppercase, or slash-bearing runtime identifiers outside the published pattern; and
- an unknown top-level member forbidden by the published closed-object contract.

## Selected minimum repair

Commit `a5645f38eb7a768667f7a55cd7294d9fc626a3b9` keeps the public serialized `CleanupReceipt` shape and version unchanged but reconstructs deserialized input through a private `CleanupReceiptWire` with `deny_unknown_fields` plus `TryFrom` validation.

The admission predicate now requires:

- exact `CONTRACT_SCHEMA_VERSION` (`1.0.0` for this receipt);
- sandbox and network identifiers matching the published lower-case ASCII alphanumeric/hyphen grammar and 1..=64 byte bound;
- `container_removed == true` and `network_removed == true`;
- no unknown top-level fields.

The valid fixture now includes digits in both runtime identifiers so the published positive identifier grammar is exercised, while existing hostile fixtures retain empty, uppercase, slash, and oversized negatives. `terminated_at_epoch_seconds` remains `u64`, matching the schema's non-negative integer contract; zero is intentionally not rejected.

The repair does not normalize or coerce hostile input, change cleanup execution, grant a deserialized receipt backend lifecycle authority, widen the schema, or alter serialization. Runtime-issued cleanup provenance and exact acquired backend identities remain separate lifecycle controls.

## DDD / evidence ownership

`application_service` owns the Supporting-context cleanup receipt semantics exposed to consumers. Infrastructure owns concrete stop/remove observations. Core/runtime lifecycle ownership must not be inferred from arbitrary caller JSON. A cleanup receipt can report runtime-issued evidence only after its wire representation satisfies the same versioned acceptance set as the published contract.

## Rejected alternatives

1. **Leave direct derived deserialization and ask callers to validate separately.** Rejected because the public Rust type itself represents trusted cleanup evidence and would retain a wider acceptance set than the published contract.
2. **Silently clamp, normalize, lowercase, or drop unknown members.** Rejected because hostile input would be converted into apparently valid evidence.
3. **Remove `Deserialize` entirely.** Viable only if the public contract is intentionally serialization-only. The current contract and RED preserve supported reconstruction, so strict wire admission is the smaller compatibility-preserving repair.
4. **Treat a valid deserialized receipt as destructive authority.** Rejected; admission proves contract consistency, not backend ownership or provenance.

## References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Joint Task Force. (2022). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A, Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53Ar5

Serde Project. (n.d.). *Container attributes*. https://serde.rs/container-attrs.html

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

## Evidence links

- Root review finding: PR #1 review `5132552775`.
- Owner issue: #87.
- Owner PR: #88.
- Runtime type: `src/application_service/mod.rs::CleanupReceipt`.
- Published contract: `schemas/application-service-cleanup.schema.json`.
- Focused executable acceptance: `tests/application_service_cleanup_deserialization_red.rs`.
- Causal RED head: `c653a9d8ad4418b8cb217b38f9bd8e534d8528a0` / native CI `34202793075`.
- Minimum production candidate: `a5645f38eb7a768667f7a55cd7294d9fc626a3b9`.

Queued/running workflow identifiers are operational metadata and are not versioned as passing evidence. Exact candidate GREEN is recorded only after the unchanged candidate head completes its required checks.