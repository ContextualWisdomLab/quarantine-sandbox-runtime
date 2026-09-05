# Product and Technical Gap Baseline

Last reviewed on 2026-09-05 KST against dependency-root PR #1 latest test-bearing head `cefb80634bd62e775839345bd23d823d154482be`, active command-runtime PR #14 latest test-bearing head `eac6b8afe998cc34171869717b882bba4002b618`, and protected/default `develop@60a85c7633e03b425b67159ec6822c8178cf87ea`. This ledger distinguishes protected truth, active-PR implementation, checked-in RED evidence, backend-applied configuration, live effective-runtime proof, queued/cancelled checks, and post-integration protected-head evidence. Predecessor evidence never transfers to a moved head.

## Product responsibility and DDD

Quarantine Sandbox Runtime owns reusable sandbox execution, isolation-policy enforcement, resource bounds, lease/readiness/cleanup attestation, hostile-workload execution evidence, application-service isolation, and artifact-analysis evidence. `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. Podman/gVisor/containerd/Kubernetes/VM remain infrastructure adapters. Wardnet retains verdict/incident/quarantine authority. Contextual-Orchestrator and Noema retain Agent/task/tool/application authorization and capability authority. Consumers integrate through released versioned contracts/ACLs, never sibling source copies, mutable PR heads, cross-service SQL, or direct ownership of backend internals.

Current structural gaps remain bounded: issue #8 owns the future durable admission/session/recovery separation; persistence has no authority yet and requires a dedicated ADR/3NF/recovery contract before introduction; the repository name remains broader in responsibility than its security-biased name and is a pre-GA product-setting decision rather than a source-layout workaround.

## Application-service isolation

The active foundation keeps digest-pinned/no-pull images, rootless execution, read-only rootfs, bounded `/tmp`, capability drop, no-new-privileges, isolated user/PID/IPC namespaces, non-root UID/GID, CPU/RAM/PID bounds, deny-by-default networking, loopback-only publication, readiness, and cleanup as fail-closed invariants. Exact root `24526eb55cf5db48ea07079b314f7d1b676eb48d` proved Podman 4.9.3/rootless behavior on hosted Ubuntu and then correctly failed closed at `IsolationVerificationFailed { control_name: "lsm" }`; that is negative effective-LSM evidence, not positive confinement. Commit `6ad2b1c9d8f616be68dc28b35d017206f26c0787` separated the ordinary hosted negative lane from the dedicated `[self-hosted, linux, cwl-hostile-workload, selinux]` positive lane.

Three P0 application-service RED descendants remain intentionally production-free until causal execution:

- #19 binds applied `/tmp` restrictions/size and wall-time configuration before any later live cgroup/mount/watchdog proof.
- #21 requires collision-resistant runtime invocation identity across independent same-request/same-second launches, separate from caller idempotency.
- #23 requires the running container's exact effective network attachment set, not merely existence of the intended internal/DNS-disabled network object.

Static Podman inspection is backend-applied configuration evidence, not proof of kernel enforcement. CPU/RAM/PID release claims still require authoritative cgroup-v2 evidence; `/tmp` claims require live mount evidence; wall-time requires behavioral termination/cleanup proof; network claims require exact attachment plus real negative-egress evidence; positive LSM claims require an eligible backend.

## Artifact analysis

The active foundation keeps SHA-256 artifact identity, bounded ingestion, deterministic format classification/evidence ordering, analyzer failure attribution, and no execution in static paths under `src/artifact_analysis/`. Draft #18 owns issue #17's Claude-plugin package-analysis contract RED while preserving quarantine-sandbox-runtime as evidence/isolation owner; AppGuardrail remains static-scan/SARIF authority and Noema remains admission/activation authority. YARA-X/capa/Ghidra/LIEF adapters, dynamic Linux/Windows detonation, and durable signed evidence remain missing commercial slices and must reuse or strengthen the Core sandbox boundary rather than execute hostile bytes in the Rust control process.

## Command-execution isolation

Draft #14 is a separate one-shot application-service contract. Production remains unchanged for issues #25–#39 until each focused RED executes for its intended cause.

- #25 pre-attestation execution — current `run_command_at` binds the consumer command as the OCI process and starts it before live seccomp/LSM/capability attestation. RED `585f3d955bddfb95f28e5918cfcadcac632589df` requires no hostile payload side effect before positive proof. GREEN needs a trusted hold/attest/release phase or equivalent; static inspect reordering is insufficient.
- #26 Linux pathname identity — RED `6a3c9f4a3c886057e1de3a2f05154f81188b58aa` requires literal `a\\b` and nested `a/b` to remain distinct through staging/digest/receipt. Production still performs a lossy backslash rewrite.
- #27 result schema — RED `9fd85c0fe4f5bd57eb718b16574145862a4a2a7b` requires strict JSON Schema 2020-12 coverage for the already-serialized optional `source_artifact_receipt`; GREEN is schema alignment only.
- #28 runtime identity width — RED `850ecd1f3c5548ba9f8eec636cd47dba0e0114fd` requires at least 128 retained bits in `qsr-cmd-*`. Production generates a 128-bit nonce but retains only 16 hex characters/64 bits.
- #29 non-UTF-8 Git pathname bytes — RED `2733bd46d054ff92861d5383fcbf68fd151e77ef` requires Unix pathname bytes such as `0xff` to survive staging/digest identity. Production still requires `Path::to_str()`.
- #30 host staging confidentiality — RED `b2ac8b81898aaf3f127dffc57cbe7b59008fa256` requires the host staging root to remain owner-only while legitimate rootless container access is provided by a reviewed backend-owned handoff. Production currently widens the staging root to `0o755`.
- #31 host log-storage bound — RED `c75d02c23567dfdc0db441589c03d2df74791bad` requires one finite canonical `k8s-file` `max-size` budget. Reader retention limits do not bound hostile log growth on host storage.
- #32 exact mount set — RED `5373af88aaacba37efea7cc764a24f1cbf4851f5` requires zero unexpected mounts without source and exactly one runtime-owned restricted `/workspace` bind with source, rejecting duplicate/conflicting destinations.
- #33 create-failure cleanup ownership — RED `313aaf694f419b6ef0020c9beae70ac37680dd7b` requires a failed create with no invocation-owned ID not to delete a same-name foreign container. Name collision probability is not ownership proof.
- #34 output encoding integrity — RED `1ab140b7c7074d47eb4dc509bdfc15c549289198` requires invalid UTF-8 stdout/stderr to fail closed instead of mutation via `String::from_utf8_lossy`. Binary-safe output requires an explicit future versioned contract.
- #35 applied tmpfs/wall-time binding — RED `83dfc0096f088febc3575e5f3dcda0c282eb734e` injects widened/executable `/tmp`, zero timeout, and request-mismatched timeout. GREEN binds exact applied configuration but does not promote it to live enforcement proof.
- #36 post-create lifecycle ownership — RED `09e4b1577d7e8df78f8d88f18226d442cd70d1e0` requires `start`/inspect/top/wait/kill/logs/remove to use the acquired long container ID rather than mutable `qsr-cmd-*` name authority after successful create.
- #37 applied immutable image identity — RED `e23bfb0e6982169b437ae13d2f1cee29c2c59754` requires exact equality between the request digest and Podman `container inspect .ImageDigest` before command output becomes trusted evidence. `.Image`/`.ImageName` are not substitutes.
- #38 direct argv versus image ENTRYPOINT — RED `387bee4ecf61de9be1915f78c2cd43b73d7379cb` requires the complete validated requested argv, including argument boundaries containing spaces, to override image-defined ENTRYPOINT semantics rather than become arguments to an unrelated image executable. `docs/doctoring/COMMAND_ENTRYPOINT_TRACEABILITY.md` records the Podman/NIST evidence chain. Production currently appends `request.command` after the image reference without an entrypoint override.
- #39 applied UTS/cgroup namespace modes — RED `eac6b8afe998cc34171869717b882bba4002b618` independently reports `HostConfig.UTSMode=host` and `HostConfig.CgroupMode=host` while all currently checked isolation evidence remains positive. Each case must fail closed before logs are trusted and clean up the invocation-owned container. Production requests `--uts=private --cgroupns=private` but does not deserialize or bind the applied inspect fields. `docs/doctoring/COMMAND_NAMESPACE_TRACEABILITY.md` records the Podman/Linux/NIST evidence chain; real namespace-handle proof remains an E2E release gate beyond inspect configuration.

The command profile must ultimately combine #25's pre-payload proof with #32/#35/#37/#39 applied-state bindings, #33/#36 ownership, #28 collision resistance, #30 host confidentiality, #31 storage bounds, #34 evidence encoding, #38 exact process argv, and real rootless-Podman E2E. No single inspect field or launch flag substitutes for that integrated proof.

## Verification and release state

Protected/default `develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea`. Issue #24 owns native CI evidence on the actual protected/default branch: its RED requires `push.branches: [develop]`, rejects stale `main`, and forbids event-specific `paths`/`paths-ignore` filters that could suppress exact integrated-head evidence. Latest test-only root authority is `cefb80634bd62e775839345bd23d823d154482be`; workflow behavior is intentionally unchanged until that RED executes.

The main command/release ancestry is `#1 → #6 → #9 → #10 → #13 → #14`, with #18 downstream of #14 through non-force adoption. RED-only #19/#21/#23 remain direct root descendants. Every moved head reacquires checks; queued, cancelled, predecessor, self-review, or static-only evidence is not release authority. Current #14 moved again for #39 and its exact-head CI must materialize and execute before any #25–#39 production GREEN is authorized. #18 must non-force adopt the moved #14 parent before its own evidence is current.

The first immutable release remains blocked until one unchanged integrated protected candidate has exact native CI, 100% owned production statement/function/region/branch and edge-case coverage where tooling exposes it, public rustdoc, real rootless isolation E2E, positive LSM/seccomp/capability/resource/network/cleanup evidence, required review/security/SAST/dependency gates, package smoke, SPDX SBOM, provenance, checksum/signature where supported, reproducibility, recovery/rollback proof, and release automation sourced from protected `develop`. GitHub Releases remain absent; mutable PR heads are not consumer authority.

## Next bounded slices

1. Execute root issue #24's current exact RED and repair the stale native `main` push trigger only after the intended failure is observed.
2. Execute #19, #23, and #21 on current root ancestry; add only their smallest causal GREENs before live resource/network proofs.
3. Execute #14 issues #25–#39 in causal order where dependencies require it; do not let a later launch-intent or applied-state fix bypass #25's pre-payload proof or #33/#36 ownership.
4. Reconcile #6/#9/#10/#13/#14 dependency-first without force, then non-force adopt the final #14 tree into #18 while preserving only #18's artifact-analysis-owned delta.
5. Merge only after one unchanged exact candidate satisfies review, security, coverage, real runtime, and protected integration gates; then publish the first immutable runtime release and hand released version/digest pinning to consumer owner paths.
