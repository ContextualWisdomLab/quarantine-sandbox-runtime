# ADR 0008: Podman-backed command execution and a CLI transport

- **Status:** Proposed
- **Date:** 2026-09-02
- **Last reviewed:** 2026-09-11

This ADR remains Proposed while PR #14 is Draft. The original pre-attestation execution defect is repaired in the active lineage, but acceptance still requires one unchanged integrated candidate with complete owned-production coverage, real rootless resource/timeout evidence, positive effective-LSM evidence, qualifying review/security gates, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback evidence.

## Context

ADR-0007 defined a bounded run-to-completion contract without a production backend or transport. The required next capability was a rootless-Podman implementation that reused the repository's isolation policy and bounded process supervision while preserving consumer-neutral contracts.

The main security constraint is temporal, not merely configurational: hostile consumer argv must not become runnable before the selected P0 isolation profile is positively established at the effective process boundary. Static `podman container inspect` state is useful contradiction evidence, but it is not proof that the running process actually has the required seccomp, capability, LSM, namespace, resource, and network confinement.

Issue #25 demonstrated the consequence. Exact historical PR #14 head `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, executed `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` and failed because the consumer command itself was already the OCI initial process when `podman start` made it runnable. A later attestation failure and cleanup cannot undo hostile code that was already allowed to execute.

A subsequent `podman init` experiment proved a useful earlier hold point, but not the complete boundary. Test-bearing `9e36766abc0a553f45c62fa327b34f1315dc0db9`, executed on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / CI `34353894509`, proved production did not yet hold and validate the created container before start. Production `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` plus formatter-only `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e` added `create -> exact ID -> init -> configured-state verification -> start`, but that remained a partial repair because the hostile consumer was still the process that would run at start.

Real backend feasibility was then established independently. Exact `7da7d3b36ec8830ab16d776e5ed1a93f7a16b8ad`, CI `34379177369`, hosted rootless-Podman job `102559489017` proved that a runtime-owned static gate can be mounted read-only as the OCI entrypoint, remain the only running process while the consumer is held, expose effective seccomp/capability/security-label evidence, retain its SHA-256 identity, and release exact consumer argv only after an explicit control event. That proof answered whether the mechanism was implementable; it did not by itself integrate the mechanism into production.

The active PR #14 lineage now performs that production composition. Historical evidence remains traceability, not current-head GREEN. No predecessor check transfers after source or documentation changes.

## Decision

### Bounded contexts and ownership

`CommandExecutionRequest` and `CommandExecutionResult` remain consumer-neutral application-service/supporting contracts. Sandbox lifecycle, isolation policy enforcement, resource bounds, attestation, timeout, and cleanup remain owned by this repository's `sandbox_execution`/infrastructure boundary. Consumer authorization, tool policy, secrets, verdicts, incidents, and business decisions stay outside this runtime behind versioned ACLs.

Podman is an infrastructure adapter. Consumers do not call Podman directly and do not own its CLI/inspection DTOs.

### Production command backend

Release callers use `RuntimeGatePodmanAdapter`. The lower-level `RootlessPodmanAdapter::run_legacy_command_at_for_test` is compiled only with debug assertions so historical/focused tests can still exercise the ungated lower-level lifecycle without making it a release surface.

The production sequence is:

```text
verify/stage immutable runtime gate
→ stage optional exact-revision source
→ verify rootless backend/security capability
→ create container with runtime gate as OCI PID 1
   while exact consumer argv is held behind a one-time token
→ acquire exact container ID from runtime-owned --cidfile
→ reject configured image/resource/tmpfs/namespace/network/mount contradictions
→ start the gate only
→ obtain and evaluate live effective seccomp/capability/LSM evidence
→ authorize one-time release scoped to the exact container ID
→ require trusted QSR_GATE_RELEASED acknowledgement
→ detach the release stdin/control channel
→ exec exact consumer argv
→ bounded wait / timeout termination / log retrieval
→ exact-ID cleanup
```

The consumer command therefore does not become the OCI initial process. Positive policy evaluation at the running gate is the only transition that can release hostile argv.

### Runtime-gate identity and loading boundary

`RuntimeGateArtifact` is a runtime-owned host artifact, not workload-image content. Admission requires:

1. an exact lowercase 64-hex SHA-256 release identity;
2. a declared architecture equal to the runtime host architecture;
3. a no-follow regular-file source;
4. source bytes matching the configured digest;
5. a bounded self-contained ELF64 `ET_EXEC` or `ET_DYN` program-header table;
6. at least one `PT_LOAD` and no `PT_INTERP` dependency on workload-image code; and
7. executable `e_machine` equal to the declared/host architecture.

Verified bytes are copied into a private read-only/executable staging area and that exact staged path is bound into the command container. Separately staged artifacts with identical bytes are equal only when their runtime authority identity is the same; independent private staging lifetimes must not alias by digest alone.

### Container identity and destructive authority

Before successful create, no container identity exists. Successful create writes a runtime-owned `--cidfile`; the admitted exact container ID becomes the sole post-create lifecycle/destructive authority. Generated `qsr-cmd-*` names remain correlation metadata. Malformed create stdout does not authorize `rm --force` against the generated name; cleanup uses an admitted runtime receipt or fails closed.

This decision avoids treating same-principal namespace names as ownership proof.

### Command isolation profile

The command sandbox requires a digest-pinned image, no pull, read-only root filesystem, explicit bounded `/tmp`, non-root numeric identity, all-capability drop, no-new-privileges, isolated user/PID/IPC/UTS/cgroup namespaces, bounded CPU/RAM/PID/lifetime, no service publication, and `--network none`.

Configured command state must include a nonzero exact `Config.Timeout`, exactly one hardened `/tmp` tmpfs matching the requested size and `rw,noexec,nosuid,nodev`, the expected resource limits, the expected image identity, required namespace state, and no network attachment. These checks are early contradiction gates. They are never relabeled as kernel-enforcement proof.

The runtime obtains live effective process evidence after starting only the gate. Unsupported/malformed evidence fails closed. Podman top/process-security evidence is treated as a backend ACL rather than a display-string assumption; unavailable positive LSM remains incomplete evidence rather than success.

### Timeout and output

Completion is observed with bounded `podman wait`. On lease expiry, the runtime requires `podman kill` success before post-kill wait evidence can be accepted; an ignored kill is a runtime failure, not a timeout result. Output is collected after exit through a finite retaining `k8s-file` log path and bounded by the command output contract. Administrative Podman timeout, output-bound overflow, invocation failure, malformed evidence, or cleanup failure remains a runtime error; nonzero workload exit is a structured workload result.

### Optional exact-revision source transport

A source input is accepted only as the complete tuple of absolute trusted host directory, canonical lowercase Git SHA-1/SHA-256 revision identity, and expected canonical tree SHA-256. The consumer path is never mounted directly.

The runtime:

- checks the caller-supplied root with no-follow metadata before canonicalization and requires root device/inode continuity across that initial resolution;
- walks only regular directories/files, rejects symlinks and special entries, and preserves exact Unix pathname bytes including literal backslashes and non-UTF-8 names;
- bounds regular-file count and total bytes;
- hashes sorted pathname length/path bytes/content length/content bytes;
- strips all executable bits in the staged copy;
- keeps the host staging root owner-only; and
- mounts only the staged tree at `/workspace` as `ro,noexec,nosuid,nodev`.

The current root-object repair does not claim that every later pathname traversal operation is fully race-free against concurrent same-principal mutation. A stronger descriptor/capability-based traversal is a separate hardening decision if required; the expected tree digest still binds accepted bytes and paths.

### Transport

The synchronous `quarantine-sandbox-runtime run` CLI uses direct argv after `--`. It validates against operator policy, requires the production runtime-gate identity for release operation, invokes the production backend, emits structured JSON on success, and does not add an HTTP listener or shell-string parser.

## Constraints and evidence semantics

OCI lifecycle ordering and Podman's process model require a distinction between pre-start configuration and effective post-start process state. A release decision must therefore retain all three evidence classes without conflating them:

- **immutable-input evidence:** image/gate/source digest and architecture/path identity;
- **configured-state evidence:** create args and exact-container inspection that can reject contradictions before release;
- **effective-runtime evidence:** live process/kernel facts sampled while only the trusted gate is runnable.

Fake-Podman/process fixtures are appropriate for deterministic lifecycle/error/ownership contracts but cannot substitute for real rootless isolation evidence. Positive LSM, cgroup-v2, live tmpfs, timeout, and negative-egress claims require a real backend that can actually demonstrate them.

## Alternatives considered

- **Static inspection alone before start:** rejected because configured state is not effective-runtime proof.
- **Treat `podman init` plus static inspection as complete issue #25 repair:** rejected because the final consumer process security state is not proven before hostile argv becomes runnable.
- **Start the hostile consumer and clean up on failed attestation:** rejected because cleanup cannot retract execution that already happened.
- **Start an inert image process then use ordinary `podman exec`:** rejected unless an equivalent pre-exec hold/attest/release boundary is proven for the exec process; merely moving the race does not remove it.
- **Use OCI `createRuntime`/`createContainer` hooks as the sole attestation point:** rejected because those stages do not guarantee all cgroup/SELinux/AppArmor process state is final.
- **Use `startContainer` hook support without a runtime-owned attestor:** rejected because hostile image content cannot become the isolation authority.
- **Use a workload-supplied gate:** rejected because the workload would then control the security mechanism that is supposed to constrain it.
- **Treat generated container names as ownership:** rejected after create because same-principal names are correlation identifiers, not an authoritative backend receipt.
- **Mount the consumer checkout directly:** rejected because mutable files, links, executable modes, and host pathname authority would cross the trust boundary without exact staged identity.
- **First transport as an HTTP service:** rejected because the present contract is one-shot and does not need listener/session semantics.
- **Consumer-owned Podman calls:** rejected because they duplicate isolation policy and backend-specific security logic outside the canonical runtime.

## Remaining risks and follow-up

The production composition closes the historical direct-consumer pre-attestation defect, but the Draft is not release-ready. Remaining release evidence includes:

- 100% owned-production statement/function/region/branch coverage without exclusions or denominator games;
- real rootless cgroup-v2 CPU/RAM/PID enforcement evidence;
- live `/tmp` mount/options/size evidence;
- behavioral wall-time kill/wait/cleanup evidence through `RuntimeGatePodmanAdapter`;
- dedicated positive effective-LSM execution on an eligible SELinux runner;
- negative egress and runtime-owned resource leak checks on the same candidate;
- fresh qualifying review/thread and central security/dependency checks;
- protected-head CI;
- version/CHANGELOG/package smoke;
- immutable tag/package or equivalent publication plus SBOM, provenance, reproducibility, and rollback evidence.

The active exact head must reacquire these checks after every source or documentation move. Queued, skipped, predecessor, static-only, fake-backend-only, or locally asserted evidence is not release authority.

## Consequences

The design adds a runtime-owned executable and a bounded control/acknowledgement channel, so the command path is more complex than direct `podman start`. That complexity is accepted because it creates an explicit security state transition: hostile code is held while the runtime samples the final running isolation boundary.

The design also makes backend ownership stricter. Exact IDs and runtime receipts must propagate through wait/log/kill/cleanup instead of reusing friendly names. This increases test surface but prevents correlation metadata from silently becoming destructive authority.

Source staging adds host I/O and hashing cost. That cost is bounded and accepted because untrusted mutable source must not become executable host/container authority. Performance claims must measure actual staging and container execution rather than bypassing this contract.

## Verification rule

Keep the historical #25 payload-side-effect RED and init-hold RED as causal evidence. Current acceptance is stronger: on one exact candidate, tests must prove consumer argv cannot become runnable before positive effective attestation, gate identity/architecture/loading are immutable and self-contained, release is one-time and exact-ID scoped, acknowledgement precedes consumer execution, stdin/control ownership is separated, timeout kill is enforced, and cleanup leaves no runtime-owned container.

Real backend verification must prove the effective security boundary rather than infer it from inspect JSON. Issue #35 therefore remains open for cgroup-v2/tmpfs/wall-time enforcement evidence and issue #43 remains coupled to the real production-gated timeout witness. The dedicated positive-LSM job must actually execute; runner unavailability is a blocker, not a passing result.

## Traceability

- `src/infrastructure/podman.rs`
- `src/infrastructure/podman_runtime_gate_binding.rs`
- `src/infrastructure/runtime_gate_artifact.rs`
- `src/bin/qsr_runtime_gate.rs`
- `src/pr_source_artifact.rs`
- `tests/podman_command_execution_e2e.rs`
- `tests/command_gate_loading_boundary_red.rs`
- `tests/podman_command_execution_gate_ack_red.rs`
- `tests/podman_command_execution_resource_config_red.rs`
- `docs/doctoring/COMMAND_PRE_ATTESTATION_TRACEABILITY.md`
- `docs/doctoring/COMMAND_HOLD_GATE_BINDING_TRACEABILITY.md`
- `docs/doctoring/COMMAND_HOLD_ATTEST_RELEASE_TRACEABILITY.md`
- `docs/doctoring/COMMAND_GATE_LOADING_BOUNDARY.md`
- `docs/product-technical-gap-baseline.md`

Authoritative standards/references and APA 7th bibliographic details remain centralized in the linked doctoring/TRACEABILITY documents. Any future reference update must distinguish historical evidence date/version from the currently consulted standard version rather than silently rewriting old causal evidence.
