# Runtime gate mount attestation traceability

## Problem

The release-authorized command path binds the independently verified runtime gate artifact read-only at `/qsr-runtime-gate` and selects it as OCI PID 1. The effective container inspection verifier nevertheless counted only the optional `/workspace` source-artifact bind. A real Podman `container inspect` record containing the required runtime-owned gate bind was therefore rejected as `command_mount_set` before the one-time release boundary could open.

This is an evidence-model defect, not permission to widen the mount set. The runtime must positively admit exactly the bind it owns while continuing to reject additional or substituted host mounts.

## Executed causal RED

PR #112 exact `3c5f9947fa2ca748171791eea2343537cff3bd12`, CI run `34621056681`, job `103334846711`, passed exact checkout, repository/CI-contract validation, and rustfmt. The workspace test suite then failed only the new `podman_runtime_gate_mount_attestation_red` target at the intended assertion:

`Backend(IsolationVerificationFailed { control_name: "command_mount_set" })`

The fixture stages a real `RuntimeGateArtifact`, emits a Podman inspection record with one exact read-only bind from that staged artifact to `/qsr-runtime-gate`, and otherwise supplies positive rootless/AppArmor/seccomp/capability/resource evidence. This proves the failure is caused by the verifier omitting the runtime-owned gate mount from its expected effective mount set.

Exact descendant `c59500f60e82de5516d2505563785d709022d3b5` independently reproduced the same causal failure in CI run `34622058959`, verify job `103338185807`: all 41 library tests and the surrounding command/runtime-gate targets passed until `podman_runtime_gate_mount_attestation_red`, which failed with the same `IsolationVerificationFailed { control_name: "command_mount_set" }`. This second execution rules out the earlier RED being an incidental runner or fixture failure.

## Source-fix workflow RCA

The first temporary source-fix workflow did not execute a job after exact `c6d699cf26c142491e49f099e32f2b0420abc4ec`. Actions run `34623208980` completed immediately with no generated jobs. Repository inspection showed that an embedded replacement shell script escaped the YAML `run: |` indentation, making the workflow definition invalid before runner assignment. This is a workflow-source defect rather than runner starvation and does not count as repair evidence.

Exact descendant `97f3a30c6629f3883167f3f53c173b6195ec0af0` adds a corrected temporary v2 workflow with the same single-writer/exact-head guards. It keeps the production repair minimal, validates the focused causal witness plus the CLI/runtime-gate integration, then runs the full workspace tests, Clippy, rustdoc, repository validation, and `git diff --check`. On success it removes both temporary source-fix workflows in the same ordinary descendant before publishing. The invalid workflow is retained only until that validated descendant can remove it without bypassing the causal verification path.

## Repair contract

The minimum repair must carry the verified staged gate path from `RuntimeGatePodmanAdapter` into both pre-start configuration verification and live process isolation verification. Effective mount admission must then require:

- mount cardinality equal to the optional source-artifact bind plus the runtime-owned gate bind;
- `/qsr-runtime-gate` present exactly within that bounded set;
- bind type `bind`;
- exact host source path equal to the already verified staged `RuntimeGateArtifact` path;
- read-only effective mount state;
- existing `/workspace` source-artifact source/read-only/noexec/nosuid/nodev checks unchanged;
- all unexpected additional mounts rejected.

Configured `--volume` arguments alone are not sufficient evidence because this boundary is explicitly attesting effective runtime state. The fake integration fixture must model the same gate mount exposed by real Podman rather than omitting it.

## Release impact

This defect is release-blocking for the command runtime. A fake-runtime integration that omits a mandatory production bind can be GREEN while the real effective-state verifier rejects the production composition. The repair is not merge or release authority until the causal RED turns GREEN on an exact descendant and ordinary verify, coverage, effective-LSM, real-runtime acceptance, review/security, and immutable release gates are satisfied.
