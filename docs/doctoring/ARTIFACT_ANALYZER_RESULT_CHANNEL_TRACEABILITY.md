# Artifact Analyzer Result-Channel Traceability

Status: RED-only design/verification evidence for issue #50. This document does not authorize production behavior or release claims.

## Problem and bounded-context ownership

`artifact_analysis` owns analyzer/profile/evidence semantics. `sandbox_execution` owns backend-neutral isolation and lifecycle evidence. Infrastructure adapters own concrete Podman/gVisor/containerd/VM worker execution. ADR-0009 keeps those responsibilities separate.

On the current #18 ancestry, `StaticAnalyzer::analyze` returns an owned `Vec<AnalyzerFinding>` and `AnalysisEngine::analyze_bytes` accepts the complete vector, normalizes every finding into controller-owned `EvidenceRecord` values, and only then runs final `EvidenceBundle::validate()`. Individual records are bounded, but aggregate finding count, aggregate normalized evidence count, and aggregate result bytes are not bounded before controller allocation/accumulation.

Issue #49 removes ambient host capability from analyzer execution. Issue #50 is independent: an isolated worker can still deny service to the trusted controller if its result transport is accepted as an unbounded document/vector or if limits are enforced only after materialization.

## Authority

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

NIST SP 800-190 treats resource allocation and workload isolation as runtime security controls. The worker boundary must therefore constrain not only worker CPU/RAM/PID/time/storage but also the data channel through which an untrusted workload can consume controller resources.

MITRE. (2026). *CWE-400: Uncontrolled resource consumption* (CWE 4.20). https://cwe.mitre.org/data/definitions/400.html

CWE-400 classifies failure to control allocation/maintenance of limited resources as uncontrolled resource consumption. The result channel is a limited controller resource boundary, not merely an evidence-format concern.

Serde Developers. (2026). *StreamDeserializer* (`serde_json` 1.0.151 documentation). https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html

`serde_json::StreamDeserializer` demonstrates incremental decoding of self-delimiting JSON values and exposes a byte offset. This is implementation evidence that Rust can normalize bounded records incrementally. It does not by itself provide an operator-owned byte/count ceiling, termination, provenance, or cleanup. Likewise `serde_json::from_reader` avoids buffering the entire input byte stream but can still materialize an attacker-sized target value such as a top-level `Vec`; using it with an unbounded aggregate is not sufficient GREEN evidence.

## RED

Test-bearing commit `618f3dae4fa7909a8725ab67895375abebba6339` adds `tests/artifact_analysis_result_channel_budget_red.rs` on a dedicated descendant of #18 so #18's queued #49 exact-head evidence is not invalidated.

The fixture uses the 65,536-byte `maximum_output_bytes` value already specified by issue #17's versioned `claude_plugin_package_analysis` contract. A purpose-built analyzer amplifies a small immutable artifact into 512 individually schema-valid `StaticCapability` findings with bounded summaries and attributes. Current production accepts the complete `Vec`, accumulates all records, validates them individually, and can return `RuntimeDisposition::Completed` even though the serialized result exceeds the declared profile-output budget.

The 65,536-byte value is fixture/profile authority, not a proposed global runtime constant. When issue #17's profile contract becomes executable, the RED must consume that versioned policy directly rather than duplicating the fixture value.

## Smallest causal GREEN after prerequisite RED execution

1. Keep analyzer execution behind ADR-0009's capability-denying worker boundary.
2. Bind a versioned profile execution policy to the exact analyzer invocation, including maximum result bytes and any independently justified record/frame count bound.
3. Enforce those bounds while reading/framing/normalizing worker output, before an attacker-sized aggregate value is materialized in the controller.
4. Stop ingestion and terminate/clean up the exact worker on overflow. Do not return a truncated/partial result as `Completed` evidence.
5. Return a bounded attributable failure/receipt tied to artifact SHA-256, analyzer/profile identity, runtime release and worker invocation; do not retain unbounded raw output.
6. Preserve deterministic record ordering and per-record validation within the aggregate budget.

Rejected alternatives:

- a magic repository-wide finding count chosen only to make the test pass;
- deserializing a complete top-level array/`Vec` and checking its size afterward;
- allowing worker OOM/kill to stand in for controller-side result-channel enforcement;
- silently truncating findings while reporting completion;
- increasing controller memory as the primary mitigation.

## Completion evidence

#49 host-capability isolation and #50 bounded result ingestion must both be GREEN on one unchanged integrated candidate. Hostile byte/count amplification, malformed framing, an oversized single record, truncated stream, worker timeout/termination, and cleanup failure need independent tests. Release evidence additionally requires exact protected-head CI, owned production rustdoc/test/edge coverage policy, security/SBOM/provenance/reproducibility, and real isolation evidence. Mutable PR heads are not consumer authority.
