# Artifact tool-failure disposition traceability

## Problem

Artifact-analysis owns execution-completeness evidence even though maliciousness/admission verdicts remain consumer-owned. `RuntimeDisposition::Completed` means every analyzer required by the requested profile completed, while `EvidenceKind::ToolFailure` denotes an attributable analyzer or worker failure.

At parent `#18@775073e912406b64ef91b805153bd699caae40eb`, `AnalysisEngine::analyze_bytes` already emits `Inconclusive` when an analyzer fails, but `EvidenceBundle::validate()` validated `disposition` independently from the evidence records. A reconstructed or deserialized bundle could therefore contain `tool_failure` evidence and still validate as `completed`. The checked-in Draft 2020-12 JSON Schema had the same semantic gap.

This is a contract-validation defect, not authority transfer. Consumers still decide whether evidence authorizes admission, but the runtime must not overstate whether its own required analysis completed.

## Executed RED authority

Issue #56 is carried by `tests/artifact_analysis_tool_failure_disposition_red.rs`.

Exact `1277495339d904327736dcfeef072b6a4a17e5e0` executed in CI run `34057626791`, branch-coverage job `101552305682`. The fixture produced the intended split: `ToolFailure + Inconclusive` passed, while both Rust validation and executable schema semantics incorrectly accepted `ToolFailure + Completed`, causing the two targeted assertions to fail. The same run's verify job `101552305744` independently exposed rustfmt drift in the test; that formatting defect did not prevent the branch-coverage lane from compiling and executing the causal RED.

The RED requires three boundaries:

1. `ToolFailure + Inconclusive` remains representable as useful partial evidence;
2. the same otherwise-valid bundle cannot validate as `Completed`;
3. the JSON Schema executes the same wire invariant rather than merely containing inert descriptive text.

## Causal repair

The repair is deliberately cross-layer because both public validators admitted the contradictory receipt.

- `b51647180de8a6e15cfd357593d79164609e9e72` applies the exact rustfmt changes reported by verify without changing test semantics.
- `941676165a1406b8fdf560fbfd03df43ad492cdd` adds a Draft 2020-12 logical guard: a receipt cannot simultaneously have `disposition = completed` and an evidence item whose `evidence_kind = tool_failure`.
- `9f62d9e064b10ea436eba3730d52288780fe9cf7` adds the equivalent Rust invariant to `EvidenceBundle::validate()` and a dedicated `ContractError::CompletedDispositionContainsToolFailure` failure class.
- `93fdf83433ac04b490499f83e8e46e2d1693f1f2` binds the Rust regression to that dedicated error instead of accepting any unrelated validation failure.

The repair does not rewrite `Inconclusive` as `Failed`, infer maliciousness, or move consumer verdict authority into the runtime. #52 profile/execution/disposition semantics remain independently enforced; #49 containment, #50 bounded worker-result ingestion, and #54 analyzer provenance remain separate release gates.

The schema change tightens `1.0.0` to the execution-completeness meaning already documented by `RuntimeDisposition::Completed` and already emitted by `AnalysisEngine` when tools fail. It is treated as an integrity correction, not a new maliciousness/admission semantic. Immutable release and consumer compatibility still require exact-head GREEN and normal protected integration before publication.

## Standards and primary references

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

JSON Schema. (2022). *JSON Schema validation: A vocabulary for structural validation of JSON (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-validation

Joint Task Force. (2020; Release 5.2.0 updated 2025). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5), AU-10 and SI-7. National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

The NIST control catalog is supporting integrity/assurance rationale. Repository semantics and executable tests remain the direct authority for what `Completed` and `ToolFailure` mean.

## Evidence chain

`AnalysisRequest` / requested profile → exact artifact digest → required analyzer/worker execution → attributable evidence records → absence/presence of `ToolFailure` → runtime disposition → Rust validation + JSON Schema validation → consumer evidence ingestion.

A consumer-visible receipt is not complete if that same receipt records failure of a required analysis tool. Current repair commits remain candidate evidence until the latest exact head executes through repository policy, tests, Clippy, rustdoc, coverage, security/review, and protected integration.
