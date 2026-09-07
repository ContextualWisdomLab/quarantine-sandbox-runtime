# Application-Service Lease Deserialization Traceability

## Decision state

Proposed security-contract repair for issue #84. The test-bearing RED is `7a9b9c3b0302377fe8a5d79780aee41b19f73477`, based directly on root exact `17e79aab32aa75efd26bcccf92415474af9bc69b`. Production behavior is intentionally unchanged until the focused RED executes for the invariant-bypass cause.

## Problem

`ServiceEndpoint`, `IsolationAttestation`, and `ApplicationServiceLease` are public evidence types. Their crate-owned construction paths establish security and lifecycle invariants: the endpoint is loopback-only, P0 isolation controls are asserted only after runtime verification, and lease metadata is assembled from the validated request plus runtime-owned metadata.

All three types also derive public Serde `Deserialize`. Deserialization assigns fields directly and does not call `ServiceEndpoint::loopback`, `IsolationAttestation::p0`, or `ApplicationServiceLease::new`. Private Rust fields therefore do not protect the wire boundary: hostile JSON can currently materialize the same public evidence types with values that runtime construction would never issue.

## DDD boundary

- `application_service` owns the consumer-facing service lease/evidence contract and its semantic admission invariants.
- `sandbox_execution` owns runtime isolation/lifecycle truth used to construct the lease.
- Infrastructure observes and enforces backend-specific runtime facts.
- A deserialized lease remains evidence/correlation data; it never becomes authorization to choose destructive runtime resources.

The repair must validate the wire-to-domain transition rather than treating Serde syntax success as domain authority.

## RED

`tests/application_service_lease_deserialization_red.rs` requires:

1. hostile `ServiceEndpoint` JSON with `host != 127.0.0.1` to fail deserialization;
2. P0 attestation JSON with any required control false, or `credentials_available=true`, to fail;
3. the current valid lease shape to remain readable if public deserialization remains supported;
4. unsupported lease schema, zero endpoint port, and non-increasing lease chronology to fail.

Current derived deserialization cannot satisfy those negative cases.

## Minimum causal GREEN

After exact RED execution, either make the evidence types serialization-only if public reconstruction is not part of the versioned contract, or deserialize through explicit wire DTOs and validated `TryFrom` conversions. The latter must reconstruct nested endpoint and attestation values through semantic checks and validate lease schema, identifiers/digests, nonzero/bounded values, and chronology before creating `ApplicationServiceLease`.

Do not silently coerce hostile values into secure defaults. Preserve the current serialized field names and schema version unless a versioned compatibility decision requires otherwise. Where the published schema forbids unknown members, the wire DTO must reject them.

## Alternatives considered

Keeping derived `Deserialize` because fields are private was rejected: visibility constrains Rust source construction, not Serde input. Post-deserialization validation left to each consumer was rejected because it creates multiple competing admission paths for one public evidence type. Coercing non-loopback or false controls to P0 defaults was rejected because it destroys evidence truthfulness. Making the lease an authorization token was rejected because resource lifecycle authority remains runtime-owned.

## Security and standards basis

NIST SP 800-53 Rev. 5.1 SI-10 requires system inputs to be checked for valid syntax and semantics, including acceptable values and numerical ranges. The hostile cases in this RED are semantically invalid even though their JSON syntax is valid. NIST SP 800-190 treats runtime isolation and container lifecycle controls as security boundaries; serialized evidence must not be able to manufacture those runtime facts. Serde's container attributes provide `deny_unknown_fields` for structural strictness, but semantic invariants still require validated domain reconstruction rather than a plain derive.

### References

Joint Task Force. (2020). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

Serde project. (n.d.). *Container attributes*. https://serde.rs/container-attrs.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Completion gate

Do not merge or mark this repair GREEN until an unchanged exact head has the causal RED followed by the minimum semantic admission repair, `cargo fmt --check`, workspace tests, clippy with warnings denied, rustdoc with warnings denied, repository validation, exact 100% owned production statement/function/region/branch coverage, required review/security/thread gates, dependency-safe parent integration, real positive effective-isolation evidence, protected-head integration, SBOM/provenance/reproducibility/rollback, and immutable release evidence.