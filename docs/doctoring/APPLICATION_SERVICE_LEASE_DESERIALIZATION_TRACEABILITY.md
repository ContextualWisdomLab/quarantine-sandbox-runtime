# Application-Service Lease Deserialization Traceability

## Decision state

Proposed security-contract repair for issue #84. Initial test-bearing RED `7a9b9c3b0302377fe8a5d79780aee41b19f73477` was based directly on root exact `17e79aab32aa75efd26bcccf92415474af9bc69b`. Review `5132181239` found that its negative matrix was narrower than issue #84 and the published `application-service-lease-1.1.0` schema. Hardened test-only RED `bccb0c1c92e288204c78096a5f4d4cd21b7cfa2c` closes that false-GREEN path. Production behavior remains intentionally unchanged until the hardened exact RED executes for the invariant-bypass cause.

## Problem

`ServiceEndpoint`, `IsolationAttestation`, and `ApplicationServiceLease` are public evidence types. Their crate-owned construction paths establish security and lifecycle invariants: the endpoint is loopback-only, P0 isolation controls are asserted only after runtime verification, and lease metadata is assembled from validated consumer intent plus runtime-owned metadata.

All three types also derive public Serde `Deserialize`. Deserialization assigns fields directly and does not call `ServiceEndpoint::loopback`, `IsolationAttestation::p0`, or `ApplicationServiceLease::new`. Private Rust fields therefore do not protect the wire boundary: hostile JSON can currently materialize the same public evidence types with values that runtime construction would never issue.

The published `schemas/application-service-lease.schema.json` is stricter than the current Rust deserializer. It closes the lease, endpoint, and attestation objects with `additionalProperties: false`; constrains request/image/backend/sandbox/network/policy identities and the policy SHA-256; requires nonzero endpoint/shutdown values; and fixes P0 attestation booleans. Rust admission must not silently widen that already-published acceptance set.

## DDD boundary

- `application_service` owns the consumer-facing service lease/evidence contract and its semantic admission invariants.
- `sandbox_execution` owns runtime isolation/lifecycle truth used to construct the lease.
- Infrastructure observes and enforces backend-specific runtime facts.
- A deserialized lease remains evidence/correlation data; it never becomes authorization to choose destructive runtime resources.

The repair must validate the wire-to-domain transition rather than treating Serde syntax success as domain authority.

## Hardened RED

`tests/application_service_lease_deserialization_red.rs` now requires all of the following before production repair:

1. hostile `ServiceEndpoint` JSON with `host != 127.0.0.1`, zero port, or an unknown member fails deserialization;
2. P0 attestation with any required control false, `credentials_available=true`, or an unknown member fails;
3. the current valid lease shape remains readable if public deserialization remains supported;
4. unsupported lease schema, zero shutdown grace, and non-increasing lease chronology fail;
5. request/policy identifiers preserve existing non-empty, bounded, control-free semantics;
6. image reference remains an immutable lower-case SHA-256 digest reference;
7. backend, sandbox, and network identifiers satisfy the already-published bounded lower-case patterns;
8. `policy_sha256` is exactly 64 lower-case hexadecimal characters;
9. an unknown top-level lease member fails because the published schema declares a closed object.

The current derived deserialization cannot satisfy these negative cases. Unknown-field checks are explicit because Serde ignores unknown fields in self-describing formats such as JSON unless `deny_unknown_fields` is present.

## Minimum causal GREEN

After exact RED execution, either make the evidence types serialization-only if public reconstruction is not part of the versioned contract, or deserialize through explicit wire DTOs and validated `TryFrom` conversions. The latter must reconstruct nested endpoint and attestation values through semantic checks and validate lease schema, identities/digests, nonzero/bounded values, closed-object semantics, and chronology before creating `ApplicationServiceLease`.

Do not silently coerce hostile values into secure defaults. Preserve the current serialized field names and schema version unless a versioned compatibility decision requires otherwise. This repair aligns the Rust ACL to the existing `1.1.0` acceptance set; it does not by itself require a schema-version increment.

## Alternatives considered

Keeping derived `Deserialize` because fields are private was rejected: visibility constrains Rust source construction, not Serde input. Adding only `deny_unknown_fields` was rejected as incomplete because closed-object syntax does not re-establish loopback, P0 boolean, identity/digest, bound, or chronology semantics. Post-deserialization validation left to each consumer was rejected because it creates multiple competing admission paths for one public evidence type. Coercing non-loopback or false controls to P0 defaults was rejected because it destroys evidence truthfulness. Making the lease an authorization token was rejected because resource lifecycle authority remains runtime-owned.

## Security and standards basis

NIST SP 800-53 SI-10 requires checking system inputs for valid syntax and semantics, including character set, length, numerical range, and acceptable values. NIST finalized SP 800-53 Release 5.2.0 on 2025-08-27; the current release remains the catalog authority for this decision. NIST SP 800-190 treats container runtime isolation and lifecycle controls as security boundaries, so serialized evidence must not be able to manufacture those runtime facts. Serde documents that `#[serde(deny_unknown_fields)]` rejects unknown fields while the default behavior for self-describing formats such as JSON ignores them; structural strictness therefore needs an explicit policy, and semantic invariants still require validated domain reconstruction.

### References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

National Institute of Standards and Technology. (2025, August 27). *NIST releases revision to SP 800-53 security and privacy controls*. https://csrc.nist.gov/News/2025/nist-releases-revision-to-sp-800-53-controls

Serde project. (n.d.). *Container attributes*. https://serde.rs/container-attrs.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Completion gate

Do not merge or mark this repair GREEN until one unchanged exact head has executed the hardened RED for the intended deserialization-bypass cause, followed by the minimum semantic admission repair and exact-head `cargo fmt --check`, workspace tests, clippy with warnings denied, rustdoc with warnings denied, repository validation, exact 100% owned production statement/function/region/branch coverage, required review/security/thread gates, dependency-safe parent integration, real positive effective-isolation evidence, protected-head integration, SBOM/provenance/reproducibility/rollback, and immutable release evidence.
