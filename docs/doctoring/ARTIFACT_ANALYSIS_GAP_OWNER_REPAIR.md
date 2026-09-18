# Artifact-analysis gap-owner repair

## Authority boundary

PR #18 remains the artifact-analysis contract/isolation parent above the command-runtime foundation. It owns issue #17 package-analysis contract semantics and issue #49's analyzer-isolation admission boundary while preserving quarantine-sandbox-runtime as the canonical isolation/evidence owner. AppGuardrail remains static-scan/SARIF authority, Noema remains admission/activation authority, and Wardnet remains verdict/incident authority.

It does not own repository-wide `docs/product-technical-gap-baseline.md`; live Gap authority remains #121. It also does not claim that a documentation-only ownership repair adopts the current command-runtime production tree. The #18 branch still has overlapping shared/source ancestry against canonical #14 and requires a deliberate ordinary non-force source reconciliation before it can become mergeable.

## Retained causal evidence

Issue #49 is a causally executed P0 artifact-analyzer isolation defect. On descendant exact `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI `34018611889`, coverage job `101446928192`, `tests/artifact_analysis_host_capability_red.rs` proved that an externally supplied `StaticAnalyzer` could reach a runtime-host loopback listener before the controller emitted no-network/no-credential/no-execution evidence. The executed failure was: `static analyzer reached a runtime-host network socket; the host-process trait call is not an isolation boundary`.

The first production repair deliberately fails closed rather than pretending an isolated worker already exists. `AnalysisEngine::new` retains validation/representation of externally supplied analyzers, while `AnalysisEngine::analyze_bytes` returns typed `IsolatedAnalyzerWorkerRequired` before invoking them inside the controller process. Runtime-owned bundled non-executing format analysis remains available through `with_bundled_static_analyzers`.

ADR-0009 remains Proposed. Commercial completion still requires the backend-neutral Analyzer Worker Execution port, a concrete capability-denying/rootless adapter, exact worker/analyzer/artifact/policy provenance, denied loopback/external network and ambient credential/environment access, bounded CPU/RAM/PID/time/storage/output, termination/cleanup/recovery evidence, and protected exact-head release evidence. A worker receipt or hard-coded no-network/no-credential boolean is not enforcement proof.

The artifact-analysis descendants remain independent contracts and are not closed by the fail-closed controller repair. In particular:

- #50/#51 bound worker-result ingestion before trusted-controller accumulation.
- #52/#53 bind dynamic execution/completeness and `RuntimeBehavior` evidence.
- #54/#55 require stable analyzer provenance rather than display-ID identity.
- #56/#57 forbid `ToolFailure` from coexisting with `Completed`.
- #58/#59 bind nested `ArtifactIdentity` subject SHA-256 to the top-level artifact descriptor.
- #69/#70 strengthen the future Analyzer Worker boundary so Core rootless/read-only-rootfs/capability-drop/no-new-privileges/user-namespace/seccomp/LSM controls must be verified before worker evidence is trusted.

Worker-dependent siblings remain Draft until the #49 prerequisite can actually execute analyzers behind an enforceable boundary. No sibling may fall back to controller-process execution merely to make tests pass.

## Single-writer decision

Review `5230295728` found that #18 still changed the repository-wide Gap ledger even though #121 owns that file. The #18 delta contained the issue #49 causal evidence and artifact-owner successor context above, so deleting it without migration would lose valid owner history.

The repair is therefore migration-first:

1. preserve #18-specific issue #17/#49 evidence and successor boundaries in this owner-local record;
2. restore only `docs/product-technical-gap-baseline.md` byte-for-byte to exact #18 base blob `92d1f3b7b2589efd034c5480c9cf842a2a5baf18` from base `4a5369de53e0b4c3b2ba446a134b5254098e7aae`;
3. leave the larger #14 production/source restack explicit and unresolved until every overlapping artifact-analysis delta can be deliberately adopted/adapted without force/rebase or tree overwrite.

This ownership repair changes no artifact-analysis production behavior, schema, public contract, fixture, or test semantics. Historical RED/GREEN evidence never transfers to the moved head. The resulting exact head must reacquire its own verification, complete owned-production coverage, review/security, applicable rootless/positive-LSM evidence and protected integration before merge or immutable release authority can advance.
