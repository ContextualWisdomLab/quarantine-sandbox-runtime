# Artifact tool-failure disposition traceability

## Problem

Artifact-analysis owns execution-completeness evidence even though maliciousness/admission verdicts remain consumer-owned. `RuntimeDisposition::Completed` means every analyzer required by the requested profile completed, while `EvidenceKind::ToolFailure` denotes an attributable analyzer or worker failure.

At parent `#18@775073e912406b64ef91b805153bd699caae40eb`, `AnalysisEngine::analyze_bytes` already emits `Inconclusive` when an analyzer fails, but `EvidenceBundle::validate()` validates `disposition` independently from the evidence records. A reconstructed or deserialized bundle can therefore contain `tool_failure` evidence and still validate as `completed`. The checked-in Draft 2020-12 JSON Schema has the same semantic gap.

This is a contract-validation defect, not authority transfer. Consumers still decide whether evidence authorizes admission, but the runtime must not overstate whether its own required analysis completed.

## RED authority

Issue #56 is carried by `tests/artifact_analysis_tool_failure_disposition_red.rs`.

The test requires three boundaries:

1. `ToolFailure + Inconclusive` remains representable as useful partial evidence;
2. the same otherwise-valid bundle cannot validate as `Completed`;
3. the JSON Schema contains an executable logical guard expressing the same wire invariant without prescribing one exact schema-composition syntax.

Production Rust and the JSON Schema remain unchanged until the exact checked-in RED executes for the intended cross-field cause.

## Causal GREEN boundary

After causal RED execution, the smallest repair must make Rust and wire validation semantically equivalent:

- any attributable `ToolFailure` precludes `Completed`;
- `Inconclusive` remains the ordinary partial-evidence state;
- `Failed` is not repurposed merely to satisfy the test;
- #52 profile/execution/disposition semantics remain independently enforced;
- #49 containment, #50 bounded worker-result ingestion, and #54 analyzer provenance remain separate release gates.

If tightening schema `1.0.0` is incompatible for existing consumers, publish a new explicit wire version with compatibility tests rather than silently changing meaning.

## Standards and primary references

JSON Schema. (2022). *JSON Schema: A media type for describing JSON documents (Draft 2020-12).* https://json-schema.org/draft/2020-12/json-schema-core

Joint Task Force. (2020; Release 5.2.0 updated 2025). *Security and privacy controls for information systems and organizations* (NIST Special Publication 800-53 Rev. 5), AU-10 and SI-7. National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-53r5

The NIST control catalog is supporting integrity/assurance rationale. Repository semantics and executable tests remain the direct authority for what `Completed` and `ToolFailure` mean.

## Evidence chain

`AnalysisRequest` / requested profile → exact artifact digest → required analyzer/worker execution → attributable evidence records → absence/presence of `ToolFailure` → runtime disposition → Rust validation + JSON Schema validation → consumer evidence ingestion.

A consumer-visible receipt is not complete if that same receipt records failure of a required analysis tool.