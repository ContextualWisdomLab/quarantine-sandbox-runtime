# Analyzer Worker Cleanup Identity Traceability

## Decision state

Proposed security-contract repair for issue #81. The test-bearing RED is `214d234ff03cd92c040fc612a40015135e217a7e`, based directly on Draft #70 exact `86e86c9c5a13f287e341a6485e6acc5f61cee811`. Production behavior is intentionally unchanged until the focused RED executes for the missing cleanup-identity cause.

## Problem

Core `SandboxWorkerIsolationEvidence` currently binds terminal observation to an exact `SandboxWorkerTerminationEvidence.worker_id`, but represents cleanup only as `cleanup_completed: bool`. The boolean has no resource or worker identity. A backend can therefore report termination for worker A and cleanup completion for a different worker/resource B without creating a contradiction visible to `AnalyzerWorkerReceipt::validate_against`.

Cleanup is lifecycle evidence, not analyzer output. A boolean that does not identify the cleaned worker is insufficient to prove the repository invariant that runtime-owned cleanup completed for the exact isolated worker whose receipt is being admitted.

## Constraints

- `sandbox_execution` remains the Core owner of reusable worker identity, termination, cleanup, resource and isolation evidence.
- `artifact_analysis` remains the Supporting owner of analyzer request/result semantics and consumes Core lifecycle evidence through an ACL.
- Infrastructure adapters own provider-specific cleanup operations and observations.
- The repair must remain backend-neutral; Podman names, container IDs, gVisor handles, containerd task IDs or VM identifiers do not become Supporting-context domain types.
- #69/#70 seven-control isolation completeness, #77/#78 finding producer authority and #79/#80 process-exit/completion binding remain independent gates.

## RED

`tests/artifact_analysis_worker_cleanup_identity_red.rs` requires three cases:

1. terminal evidence for worker A plus completed cleanup evidence for worker B fails closed as `IsolationBoundaryViolated { field_name: "cleanup_worker_id" }`;
2. exact-worker cleanup with `completed=false` fails closed as `cleanup_completed`;
3. exact-worker cleanup with `completed=true` remains valid when the rest of the receipt is valid.

The current source cannot satisfy the contract because no Core cleanup evidence value carrying worker identity exists.

## Minimum causal GREEN

After the exact RED executes for that cause, introduce one Core value such as `SandboxWorkerCleanupEvidence { worker_id, completed }`, compose it into `SandboxWorkerIsolationEvidence`, validate the cleanup identity against the enclosing worker identity, and remove the unscoped `cleanup_completed` boolean. No analyzer taxonomy, backend selection, retry behavior, termination taxonomy or dynamic-evidence semantics should change in this repair.

Concrete adapters must later prove that their cleanup operation targeted the exact acquired runtime identity. Matching Rust values are consistency evidence, not proof that cleanup happened.

## Alternatives considered

Keeping `cleanup_completed: bool` was rejected because it cannot identify the cleaned worker. Adding a second `cleanup_worker_id` beside the boolean would make the invariant representable but would preserve two loosely coupled fields; a cleanup evidence value keeps lifecycle identity and observation together. Moving cleanup ownership into `artifact_analysis` was rejected because cleanup is reusable sandbox lifecycle truth. Embedding provider-specific container identifiers in the Supporting contract was rejected because it breaks backend neutrality and the Context Map.

## Security and standards basis

The repository's AGENTS/TRD require cleanup to fail closed and make runtime lifecycle evidence an isolation responsibility. NIST SP 800-190 treats container runtime isolation/resource controls and lifecycle security as part of the container security boundary. OCI Runtime Specification v1.3.0 defines lifecycle operations against the same container identity; the runtime user must be able to apply operations to the container it created. These sources support preserving exact runtime identity across lifecycle operations; repository-specific worker identity remains the executable contract authority.

### References

Open Container Initiative. (2025, November 4). *Open Container Initiative Runtime Specification* (Version 1.3.0). https://specs.opencontainers.org/runtime-spec/runtime/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Completion gate

Do not merge or mark this repair GREEN until an unchanged exact head has the causal RED followed by the minimum GREEN, `cargo fmt --check`, workspace tests, clippy with warnings denied, rustdoc with warnings denied, repository validation, exact 100% owned production statement/function/region/branch coverage, required review/security/thread gates, dependency-safe parent integration, real positive effective-isolation evidence, protected-head integration, SBOM/provenance/reproducibility/rollback, and immutable release evidence.