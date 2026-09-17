# Quarantine Runtime Foundation Design

## Approval

The prior architecture discussion established that a new repository is required and that Wardnet must remain the verdict and SOC control plane. The instruction to create the PR approves this bounded foundation design.

## Problem

CWL products receive untrusted files through multiple sources, but executing or deeply parsing them inside Wardnet, EgressWeave, Naruon, or a general application process would collapse privilege and failure boundaries.

## Selected approach

Create `ContextualWisdomLab/quarantine-sandbox-runtime` as an independently deployable runtime whose first vertical slice is deliberately non-executing:

1. source-agnostic request with optional typed, bounded source context;
2. bounded byte ingestion with no required file name;
3. immutable SHA-256 identity;
4. deterministic file-family evidence;
5. ordered static-analyzer port;
6. attributable failures;
7. explicit no-network/no-credentials/no-execution manifest;
8. consumer-owned verdict.

The bounded source context is correlation metadata, not payload. Its published fields are `source_channel_code`, `original_file_name`, `declared_media_type`, `host_artifact_reference`, and `submitted_at`. Unknown fields and empty present contexts fail closed, and the serialized context is capped at 1,024 UTF-8 bytes. The runtime accepts raw bytes without any source context.

## Alternatives rejected

### Put everything in Wardnet

Rejected because a long-lived SOC control plane should not share process privileges, parsers, and execution surfaces with hostile samples.

### Put everything in EgressWeave

Rejected because EgressWeave controls approved HTTP egress; it is neither a binary reverse-engineering engine nor a kernel-level network sandbox.

### Build dynamic detonation first

Rejected because Linux microVM and Windows VM execution require independent host, image, egress, telemetry, escape, and capacity contracts. A partial sandbox would be more dangerous than an explicit `inconclusive` response.

## Public interfaces

- `AnalysisRequest`
- `BoundedSourceContext`
- `AnalysisProfile`
- `IngestionPolicy`
- `ingest_bytes`
- `StaticAnalyzer`
- `AnalyzerFinding`
- `AnalysisEngine`
- `EvidenceBundle`
- `to_pretty_json`

## Data flow

```text
request validation
→ optional bounded source-context validation
→ ingestion policy
→ SHA-256 identity
→ format classification
→ analyzer findings/failures
→ deterministic evidence ordering
→ runtime boundary manifest
→ completed or inconclusive
```

## Error handling

Request and source-context failures stop before artifact processing. Ingestion failures stop before analyzer invocation. Analyzer failures are converted to `tool_failure` evidence and downgrade the disposition to `inconclusive`. Dynamic requests remain inconclusive until a matching isolated worker exists.

## Testing

The bounded source-context drift was repaired test-first: a regression test first established the missing public contract, optional-context behavior, malformed/sensitive-shape rejection, aggregate byte cap, and no-required-filename runtime behavior. Implementation and checked-in schemas then follow that executable contract. Exact-head CI enforces lint, docs, policy, and complete production coverage.

## Follow-on slices

1. YARA-X adapter.
2. capa adapter.
3. executable metadata and entropy adapter.
4. safe archive and Office/PDF extraction.
5. evidence attestation.
6. Linux disposable worker.
7. Windows disposable worker.
8. controlled network sinkhole.
9. Wardnet evidence ingestion and verdict policy.
