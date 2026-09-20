# EvidenceBundle non-cardinality baseline traceability

## Finding

Review `5261015098` on schema-cardinality owner #130 found a remaining false-GREEN outside the previously hardened `properties.evidence` subtree. A candidate could add the intended three direct foundation occurrence constraints and also change an unrelated public EvidenceBundle contract, for example by adding root `not: {}` or widening `analysis_job_id.maxLength`. The pointer-local cardinality witness would still observe the expected `(1, 1)` foundation bounds even though the published schema could reject every ordinary bundle or silently weaken another wire invariant.

This is an owner-lane integrity defect, not authority to widen #130 into general schema redesign. `artifact_analysis` still owns the EvidenceBundle contract, while #130 owns only the foundation-cardinality parity delta. Production Rust and `schemas/evidence-bundle.schema.json` remain unchanged.

## Test-first repair

Test-only commit `53f8ce621010f35a382c0fa31a443d7da632eb52` adds `tests/artifact_analysis_foundation_schema_non_cardinality_integrity_red.rs`. The guard normalizes the complete checked-in EvidenceBundle schema and removes only `properties.evidence.allOf`, the one location #130 is authorized to add. It then binds the remaining public schema to SHA-256 `e92a6562641ed7f4fc21f1d247cb97f78709a6dd8aaa34e8820c6343c1b90206`.

The positive control injects exactly the intended three foundation `contains + minContains: 1 + maxContains: 1` branches and requires the non-cardinality digest to remain unchanged. Two hostile controls then require rejection of a root-level `not: {}` assertion and an unrelated widening of `properties.analysis_job_id.maxLength` to `4096`. This closes the class of false GREENs in which local cardinality structure is correct while some other public contract changes at the same time.

The digest is a focused change detector, not a replacement for JSON Schema semantic validation. Any legitimate unrelated EvidenceBundle schema evolution must proceed through its own owner lane and deliberately re-baseline this guard with review evidence. #130 must not update the digest merely to make an unrelated schema change pass.

## Allowed next step

Keep #130 Draft. Current production schema remains intentionally without the foundation `allOf`. The moved exact must execute the existing cardinality witness plus this complete non-cardinality preservation guard. Only after the real checked-in schema reaches the intended missing-foundation-cardinality RED may the production schema add exactly the three narrow direct occurrence constraints. That GREEN must leave every other normalized public-schema byte semantic represented by this digest unchanged.

No root applicator, unrelated property change, schema-version rewrite, source copy, force update, predecessor status transfer, or release claim is authorized from this test-first movement.
