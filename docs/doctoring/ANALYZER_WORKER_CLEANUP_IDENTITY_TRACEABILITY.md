# Analyzer Worker Cleanup Identity Traceability

## Decision state

Proposed security-contract repair for issue #81. The cleanup-identity RED has executed for its intended missing-Core-value cause, and the minimum production candidate has passed all three focused cleanup-identity cases on its exact source head. The pull request is not integrated GREEN because the broad workspace still reaches an inherited command-runtime prerequisite failure, Clippy/rustdoc and complete coverage do not complete on that run, positive effective-LSM evidence is still pending, and protected/release gates remain open.

## Problem

At the causal RED head, Core `SandboxWorkerIsolationEvidence` bound terminal observation to an exact `SandboxWorkerTerminationEvidence.worker_id` but represented cleanup only as `cleanup_completed: bool`. The boolean had no resource or worker identity. A backend could therefore report termination for worker A and cleanup completion for a different worker/resource B without creating a contradiction visible to `AnalyzerWorkerReceipt::validate_against`.

Cleanup is lifecycle evidence, not analyzer output. A boolean that does not identify the cleaned worker is insufficient to prove the repository invariant that runtime-owned cleanup completed for the exact isolated worker whose receipt is being admitted.

## Constraints

- `sandbox_execution` remains the Core owner of reusable worker identity, termination, cleanup, resource and isolation evidence.
- `artifact_analysis` remains the Supporting owner of analyzer request/result semantics and consumes Core lifecycle evidence through an ACL.
- Infrastructure adapters own provider-specific cleanup operations and observations.
- The repair remains backend-neutral; Podman names, container IDs, gVisor handles, containerd task IDs or VM identifiers do not become Supporting-context domain types.
- #69/#70 seven-control isolation completeness, #77/#78 finding producer authority and #79/#80 process-exit/completion binding remain independent gates.

## Executed RED

Test-bearing `214d234ff03cd92c040fc612a40015135e217a7e` adds `tests/artifact_analysis_worker_cleanup_identity_red.rs` with three required cases:

1. terminal evidence for worker A plus completed cleanup evidence for worker B fails closed as `IsolationBoundaryViolated { field_name: "cleanup_worker_id" }`;
2. exact-worker cleanup with `completed=false` fails closed as `cleanup_completed`;
3. exact-worker cleanup with `completed=true` remains valid when the rest of the receipt is valid.

After dependency-safe adoption of formatted parent #70, exact `1e02a5e0f1cc29c3560728c7b09f8d15eceec16a` executed in native CI `34306171116`. Repository/fmt prerequisites passed and `cargo test --locked --workspace --all-targets` failed at compile time for the intended contract gap: E0432 reported missing public `SandboxWorkerCleanupEvidence`, and E0560 reported that `SandboxWorkerIsolationEvidence` had no `cleanup` field and still exposed only `cleanup_completed`. This is the causal RED; no production behavior was changed to manufacture it.

## Minimum causal repair and focused GREEN

Production `1719eeca0c634b0fa0b8fecec52215612813dfcc` introduces backend-neutral Core `SandboxWorkerCleanupEvidence { worker_id, completed }`, composes it into `SandboxWorkerIsolationEvidence`, validates the cleanup identifier shape, rejects cleanup identity mismatch as `cleanup_worker_id`, rejects incomplete cleanup as `cleanup_completed`, and removes the unscoped boolean. The public crate facade exports the new Core value. No analyzer taxonomy, backend selection, retry behavior, termination taxonomy or dynamic-evidence semantics changed.

Native CI `34307374958`, verify `102326771434`, executed that exact source head. Exact checkout, dependency lock, repository policy, coverage-parser tests and rustfmt passed. The focused suite then passed all three cleanup-identity cases: different-worker cleanup is rejected, incomplete exact-worker cleanup is rejected, and completed exact-worker cleanup is admitted. Existing Core worker-boundary, identifier, outcome, worker-port and required-isolation-control suites also passed.

The same verify job later failed in inherited `podman_command_execution_entrypoint_red::requested_command_is_encoded_as_exact_entrypoint_argv`. That failure is outside this semantic slice: the #82 ancestry does not yet contain canonical command-runtime issue #38's `--entrypoint=<JSON argv>` repair already owned by Draft #14. Hosted negative rootless/AppArmor job `102326771050` passed; coverage `102326771170` and branch coverage `102326771215` failed during broad workspace generation; positive-LSM `102326771269` remains queued. Because Test stopped before lint/documentation, Clippy and rustdoc did not execute. The focused cleanup-identity repair is therefore proven within the test step, but the PR is not merge-ready and no broad/predecessor GREEN transfers.

Concrete adapters must still prove that their cleanup operation targeted the exact acquired runtime identity. Matching Rust values are consistency evidence, not proof that cleanup happened.

## Alternatives considered

Keeping `cleanup_completed: bool` was rejected because it cannot identify the cleaned worker. Adding a second `cleanup_worker_id` beside the boolean would make the invariant representable but would preserve two loosely coupled fields; a cleanup evidence value keeps lifecycle identity and observation together. Moving cleanup ownership into `artifact_analysis` was rejected because cleanup is reusable sandbox lifecycle truth. Embedding provider-specific container identifiers in the Supporting contract was rejected because it breaks backend neutrality and the Context Map.

## Security and standards basis

The repository's AGENTS/TRD require cleanup to fail closed and make runtime lifecycle evidence an isolation responsibility. NIST SP 800-190 treats container runtime isolation/resource controls and lifecycle security as part of the container security boundary. OCI Runtime Specification v1.3.0 defines lifecycle operations against the same container identity; the runtime user must be able to apply operations to the container it created. These sources support preserving exact runtime identity across lifecycle operations; repository-specific worker identity remains the executable contract authority.

### References

Open Container Initiative. (2025, November 4). *Open Container Initiative Runtime Specification* (Version 1.3.0). https://specs.opencontainers.org/runtime-spec/runtime/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Completion gate

This doctoring update moves the PR head beyond the exact source head that produced the focused GREEN; predecessor evidence is retained only as causal history. Do not merge until one unchanged final head passes repository validation, rustfmt, full workspace tests, Clippy with warnings denied, rustdoc with warnings denied, exact 100% owned production statement/function/region/branch coverage, required review/security/thread gates, dependency-safe adoption of the repaired command/runtime foundation through #18/#70, real positive effective-isolation evidence, protected-head integration, SBOM/provenance/reproducibility/rollback, and immutable release evidence.
