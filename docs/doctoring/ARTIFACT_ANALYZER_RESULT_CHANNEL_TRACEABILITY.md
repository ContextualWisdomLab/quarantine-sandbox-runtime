# Artifact Analyzer Result-Channel Traceability

Status: RED-only design/verification evidence for issue #50. Current parent is #18 exact `7a48b7f904ce41ce2d2f184028c2eec7a9201d55`. This document does not authorize production behavior or release claims.

## Problem and bounded-context ownership

`artifact_analysis` owns analyzer/profile/evidence semantics. `sandbox_execution` owns backend-neutral isolation and lifecycle evidence. Infrastructure adapters own concrete Podman/gVisor/containerd/VM worker execution. ADR-0009 keeps those responsibilities separate.

The pre-worker analyzer contract returns an owned `Vec<AnalyzerFinding>` and normalizes every finding into controller-owned `EvidenceRecord` values before final `EvidenceBundle::validate()`. Individual records are bounded, but aggregate finding count, normalized evidence count, and result bytes have no controller-side pre-materialization bound.

Issue #49 is now causally reproduced and its first repair fails closed before externally supplied analyzer invocation. That removes the demonstrated ambient-host capability exposure, but it also means #50 cannot yet exercise its worker-result path. An `IsolatedAnalyzerWorkerRequired` error is therefore a prerequisite blocker, not #50 GREEN. The hardened RED explicitly rejects treating that #49 error as a successful bounded-ingestion outcome.

## Authority

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

NIST SP 800-190 treats resource allocation and workload isolation as runtime security controls. The worker boundary must constrain not only worker CPU/RAM/PID/time/storage but also the data channel through which an untrusted workload can consume controller resources.

MITRE. (2026). *CWE-400: Uncontrolled resource consumption* (CWE 4.20). https://cwe.mitre.org/data/definitions/400.html

CWE-400 classifies failure to control allocation/maintenance of limited resources as uncontrolled resource consumption. The result channel is a limited controller resource boundary, not merely an evidence-format concern.

Serde Developers. (2026). *StreamDeserializer* (`serde_json` 1.0.151 documentation). https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html

`serde_json::StreamDeserializer` demonstrates incremental decoding of self-delimiting JSON values and exposes a byte offset. This is implementation evidence that Rust can normalize bounded records incrementally. It does not by itself provide an operator-owned byte/count ceiling, termination, provenance, or cleanup. Likewise `serde_json::from_reader` can still materialize an attacker-sized top-level aggregate and is not sufficient GREEN evidence by itself.

## RED

Initial test-bearing commit `618f3dae4fa7909a8725ab67895375abebba6339` adds `tests/artifact_analysis_result_channel_budget_red.rs`. The fixture uses issue #17's 65,536-byte `maximum_output_bytes` example and amplifies a small immutable artifact into 512 individually valid `StaticCapability` findings.

After #49's fail-closed production repair landed on the parent candidate, the previous broad `Err(_)` acceptance became a false-GREEN path: the test could pass because no worker existed, without exercising any result channel. The current hardening distinguishes `AnalysisError::IsolatedAnalyzerWorkerRequired` and deliberately fails in that state. Other fail-closed outcomes are acceptable only after a worker/result-channel path exists and rejects the invocation for the bounded-ingestion contract.

The 65,536-byte value is fixture/profile authority, not a proposed global runtime constant. When issue #17's profile contract becomes executable, the RED must consume that versioned policy directly rather than duplicating the fixture value.

## Smallest causal GREEN after prerequisite execution

1. Keep analyzer execution behind ADR-0009's capability-denying worker boundary.
2. Bind a versioned profile execution policy to the exact analyzer invocation, including maximum result bytes and any independently justified record/frame count bound.
3. Enforce those bounds while reading/framing/normalizing worker output, before an attacker-sized aggregate value is materialized in the controller.
4. Stop ingestion and terminate/clean up the exact worker on overflow. Do not return a truncated/partial result as `Completed` evidence.
5. Return a bounded attributable failure/receipt tied to artifact SHA-256, analyzer/profile identity, runtime release and worker invocation; do not retain unbounded raw output.
6. Preserve deterministic record ordering and per-record validation within the aggregate budget.

Rejected alternatives include a magic repository-wide finding count, post-hoc validation of a complete attacker-sized `Vec`, worker OOM as controller enforcement, silent truncation, larger controller memory, and treating a missing isolated worker as proof that the result channel itself is bounded.

## Completion evidence

#49 host-capability isolation and #50 bounded result ingestion must both be GREEN on one unchanged integrated candidate. Hostile byte/count amplification, malformed framing, an oversized single record, truncated stream, worker timeout/termination, and cleanup failure need independent tests. Release evidence additionally requires exact protected-head CI, owned production rustdoc/test/edge coverage policy, security/SBOM/provenance/reproducibility, and real isolation evidence. Mutable PR heads are not consumer authority.
