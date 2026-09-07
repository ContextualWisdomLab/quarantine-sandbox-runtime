# Application-service cleanup receipt admission TRACEABILITY

Date: 2026-09-07
Status: RED-only design record for issue #87
Parent authority: root PR #1 exact `17e79aab32aa75efd26bcccf92415474af9bc69b`

## Problem

`CleanupReceipt` is a public runtime-evidence type and derives `Deserialize`, while the only invariant-establishing constructor is crate-owned `CleanupReceipt::complete`. Direct Serde construction therefore bypasses the invariants already published by `schemas/application-service-cleanup.schema.json`.

The published `application-service-cleanup-1.0.0` document is closed (`additionalProperties: false`), fixes `schema_version` to `1.0.0`, constrains sandbox/network identifiers to `^[a-z0-9-]{1,64}$`, and fixes both removal fields to `true`. The current Rust deserializer does not re-run those assertions. A private Rust field is not an admission control when the containing public type implements `Deserialize`.

This is evidence integrity, not cleanup authority. Issue #42 separately owns whether a caller-visible lease can select destructive backend targets. Issue #84 / Draft #85 owns forged lease and isolation-attestation admission. Issue #87 owns only whether serialized cleanup evidence can become the same Rust `CleanupReceipt` type without satisfying the existing cleanup schema.

## RED

`tests/application_service_cleanup_deserialization_red.rs` preserves one valid round trip and then requires deserialization failure for:

- an unsupported cleanup schema version;
- either removal claim set to `false`;
- empty, oversized, uppercase, or slash-bearing runtime identifiers outside the published pattern; and
- an unknown top-level member forbidden by the published closed-object contract.

Production is intentionally unchanged until this exact RED executes for the permissive-deserialization cause. A checked-in test is not causal execution evidence.

## Decision boundary

After causal RED, prefer the smallest repair that preserves the existing wire contract:

1. remove public `Deserialize` if cleanup receipts are intentionally serialization-only runtime evidence; or
2. keep wire reconstruction only through strict deserialization that enforces the current `1.0.0` acceptance set before materializing `CleanupReceipt`.

Do not coerce hostile values to secure defaults, silently discard unknown members, add an untyped extension map, widen the published identifier grammar, or make a deserialized receipt destructive lifecycle authority. Runtime-owned cleanup provenance and exact acquired backend identities remain separate lifecycle controls.

## DDD / evidence ownership

`application_service` owns the Supporting-context cleanup receipt semantics exposed to consumers. Infrastructure owns concrete stop/remove observations. Core/runtime lifecycle ownership must not be inferred from arbitrary caller JSON. A cleanup receipt can report runtime-issued evidence only after its wire representation satisfies the same versioned acceptance set as the published contract.

## References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53, Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Joint Task Force. (2022). *Assessing security and privacy controls in information systems and organizations* (NIST Special Publication 800-53A, Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53Ar5

Serde Project. (n.d.). *Container attributes*. https://serde.rs/container-attrs.html

Wright, A., Andrews, H., Hutton, B., & Dennis, G. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12)*. JSON Schema. https://json-schema.org/draft/2020-12/json-schema-core

## Evidence links

- Root review finding: PR #1 review `5132552775`.
- Owner issue: #87.
- Runtime type: `src/application_service/mod.rs::CleanupReceipt`.
- Published contract: `schemas/application-service-cleanup.schema.json`.
- Focused executable acceptance: `tests/application_service_cleanup_deserialization_red.rs`.

Transient queued/running workflow identifiers are operational metadata and must not be versioned into this file.
