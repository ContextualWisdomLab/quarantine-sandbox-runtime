# Artifact ingestion name invariant traceability

Status: Proposed

Authority under test: artifact-analysis parent `c0647152ec052d82969b2ae078891e25e6d4d69a`, Issue #97, branch `test/artifact-ingestion-name-invariant-red`.

## Problem

`ArtifactDescriptor` is the canonical artifact identity contract. Its validation bounds artifact names to 255 UTF-8 bytes and treats `original_file_name` as a leaf name: `.`, `..`, control characters, `/`, and `\\` are invalid.

The ingestion anti-corruption layer currently has a wider acceptance set. `IngestionPolicy::validate` rejects only zero limits, `validate_artifact_name` uses the caller-configured maximum without a canonical 255-byte ceiling, and ingestion does not apply the descriptor leaf-name invariant before constructing and returning the descriptor. A caller can therefore receive an `IngestedArtifact` whose descriptor is invalid according to the same bounded context.

This is a contract-consistency defect rather than a path-normalization feature request. Ingestion should validate the supplied leaf identity as-is; it should not strip components, truncate bytes, or rewrite caller data into a different identity.

## DDD boundary and invariant

Artifact ingestion is an admission/anti-corruption boundary for untrusted bytes plus source identity. The domain invariant is:

> Every artifact accepted by ingestion must carry an `ArtifactDescriptor` that is valid under the canonical artifact-analysis contract without post-hoc normalization.

The configured ingestion bound may be narrower than the canonical bound, but it may not widen it. This preserves policy-specific restriction while preventing a policy object from authorizing a state the aggregate contract cannot represent.

## RED

Test-only commit `cfde9d0afe8223352182f74498e14dcc6e14be8c` adds `tests/artifact_ingestion_name_invariant_red.rs` and requires:

- `maximum_artifact_name_bytes = 256` to be rejected as invalid policy rather than authorizing a 256-byte descriptor name;
- `.`, `..`, `folder/sample.bin`, `folder\\sample.bin`, and `../sample.bin` to be rejected by ingestion;
- a valid Unicode leaf name below the UTF-8 byte ceiling to remain accepted;
- an accepted artifact's descriptor to pass `ArtifactDescriptor::validate`.

Production Rust is unchanged. The test remains a checked-in RED until the exact head executes and fails for the intended acceptance-set mismatch.

## Minimum causal GREEN

After causal RED:

1. constrain `IngestionPolicy.maximum_artifact_name_bytes` to `1..=255` using the existing invalid-policy failure boundary;
2. make ingestion leaf-name admission agree with the canonical descriptor invariant for `.`, `..`, control characters, `/`, `\\`, and the configured/canonical UTF-8 byte ceilings;
3. preserve valid Unicode leaf names and existing format detection;
4. prefer a shared canonical predicate/constant where the current module dependency direction allows it, so ingestion and descriptor validation cannot drift independently.

No filename normalization, basename extraction, truncation, lossy Unicode conversion, or consumer workaround is part of this repair.

## Alternatives rejected

- **Call `descriptor.validate()` only after constructing it.** This would prevent an invalid return value but would translate a known ingestion-policy/input defect into a downstream contract error and retain two different admission languages. The boundary should reject invalid policy/input before it manufactures domain state.
- **Silently clamp configured limits to 255.** This changes operator intent and hides invalid configuration. Fail closed instead.
- **Strip directories with `basename` or equivalent.** This changes caller-supplied identity and can collapse distinct inputs. Reject non-leaf identity instead.
- **Expand the descriptor contract above 255 bytes.** That would widen the published canonical contract solely to accommodate an implementation bug and would require a versioned contract decision, not a leaf repair.

## Security and operability evidence

MITRE's current CWE-20 guidance treats filenames and metadata as untrusted input surfaces and recommends an accept-known-good strategy that validates length, syntax, consistency across related fields, and business-rule conformance. NIST SP 800-53 Rev. 5 SI-10 likewise requires checking information inputs for validity and notes that input validation supports accurate interpretation by downstream components. Here the concrete security/control objective is consistency: the ingestion boundary must not accept metadata that the domain descriptor immediately rejects.

This repair does not claim that rejecting separators alone prevents all path traversal. The artifact name is metadata, not a filesystem path capability. Filesystem staging and path ownership remain separate boundaries.

## Verification and release impact

The lane is not GREEN until the exact integrated head passes repository validation, fmt, tests, Clippy, rustdoc, exact 100% owned-production statement/function/region/branch coverage, review/security gates, parent-safe non-force integration, and the repository's protected-release evidence. Issue #97 and root PR #1's ingestion review thread remain open until that adoption occurs.

## References

MITRE. (2026). *CWE-20: Improper input validation (Version 4.20).* Common Weakness Enumeration. https://cwe.mitre.org/data/definitions/20.html

National Institute of Standards and Technology. (2020). *Security and privacy controls for information systems and organizations (NIST Special Publication 800-53, Revision 5).* U.S. Department of Commerce. https://doi.org/10.6028/NIST.SP.800-53r5
