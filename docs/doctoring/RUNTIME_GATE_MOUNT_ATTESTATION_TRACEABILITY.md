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

The repaired source-fix lane then exposed one separate test-fixture mismatch: the production gate binding uses the normal two-argument Podman form `--volume`, `<host>:/qsr-runtime-gate:ro`, while the CLI fake accepted only an invented one-token `--volume=...` spelling. That fake exited before `container create` evidence could be produced. The fixture was repaired to consume the exact two-token argv rather than changing production composition.

Run `34624104262`, job `103344906635`, executed the corrected exact-head source-fix path successfully. It passed the mount-attestation regression, runtime-gate integration, CLI success path, full `cargo test --locked --workspace --all-targets --no-fail-fast`, Clippy with `-D warnings`, rustdoc with `-D warnings`, repository validation, and `git diff --check`. It then published ordinary descendant `2a82ff32eab6cffa0b9ad41f9074d92ba72a022b` and removed both temporary source-fix workflows and both repair scripts in that same validated lineage.

The normal pull-request CI generated for the bot-authored repair commit did not execute any job: run `34624229479` terminated as `action_required`. It therefore is not exact-head GREEN evidence. This documentation descendant is intentionally user-authored so ordinary PR CI can materialize on a fresh exact head without no-op source churn; its result must be evaluated independently before merge or release.

## Implemented repair contract

`RuntimeGatePodmanAdapter` now passes the already verified staged gate artifact path into the canonical Podman command lifecycle. Both pre-start configuration verification and post-start/live isolation verification receive that path. Effective mount admission requires:

- mount cardinality equal to the optional source-artifact bind plus the optional runtime-owned gate bind;
- `/qsr-runtime-gate` present exactly within that bounded set when the gated path is used;
- gate mount type `bind`;
- exact host source path equal to the already verified staged `RuntimeGateArtifact` path;
- gate effective state read-only;
- existing `/workspace` source-artifact source/read-only/noexec/nosuid/nodev checks unchanged;
- all unexpected additional mounts rejected by cardinality before consumer release.

Configured `--volume` arguments alone are not accepted as evidence. The verifier checks the effective `container inspect` mount record, while the integration and CLI fakes now model the same mandatory gate bind that production creates.

## Release impact

The causal `command_mount_set` RED has a validated source-fix GREEN lineage, but this remains a Draft repair rather than release authority. The fresh ordinary PR head must independently pass repository/fmt/tests/Clippy/rustdoc, complete owned-production coverage, review/security, dedicated positive effective-LSM, real #35/#43 runtime acceptance, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback gates. No descendant evidence is transferred to canonical #14 until ordinary integration.
