# ADR 0008: Podman-backed command execution and a CLI transport

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while PR #14 is Draft. Acceptance requires protected integration of the prerequisite stack, a pre-payload effective-isolation boundary that makes issue #25 GREEN, fresh exact-head execution of the current cleanup and isolation regressions, effective-isolation proof on an eligible LSM-capable backend, and release-grade evidence under live governance.

## Context

ADR-0007 defines a bounded run-to-completion contract but intentionally ships no production backend or transport. The next slice needs a real rootless-Podman implementation that reuses the existing isolation policy and process supervisor, plus a minimal transport usable by CI/security consumers.

A security review corrected an earlier design idea: static `podman container inspect` configuration is evidence of requested/configured state, not positive proof of effective per-process seccomp/LSM/capability enforcement. `run_command_at` therefore fails closed when live process evidence cannot be obtained; it does not fall back to static-only attestation.

Issue #25 exposed a stronger lifecycle defect. Exact PR #14 head `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, executed `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` and failed because the consumer command was already the OCI process when `podman start` was invoked. A later live-attestation error and cleanup cannot undo hostile code that became runnable before that evidence was sampled. The previous prose saying the runtime inspected the running process before allowing the requested command to execute was therefore incorrect and is superseded by this Proposed decision.

The next RED narrowed one usable hold primitive without closing the P0. Test-bearing `9e36766abc0a553f45c62fa327b34f1315dc0db9`, executed after immutable-fixture prerequisites on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / native CI `34353894509`, proved the command path did not invoke `podman init`: an unavailable init primitive did not fail closed and invalid configured isolation was not rejected while the container was still held. Production `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` plus formatter-only `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e` now orders an acquired container ID through `podman init`, pre-start configuration verification, and only then `podman start`, while retaining the existing post-start live-process verifier. This is a minimum partial repair for the executed init-hold counterexample, not effective-process attestation.

OCI runtime lifecycle semantics explain both the value and the limit of that repair. OCI `create` establishes the runtime environment without running the user-specified program, and Podman `init` performs the work needed to start an already-created container without starting it. During `start`, OCI `startContainer` hooks execute before the user-specified process. By contrast, OCI explicitly notes that `createRuntime` and `createContainer` hooks may run before cgroups and SELinux/AppArmor setup is complete. A production pre-payload boundary must preserve these distinctions rather than promoting configured state to effective process evidence.

## Decision

### Backend

`RootlessPodmanAdapter::run_command_at` implements `CommandExecutionBackend` and reuses the existing P0 isolation policy and `BoundedCommandRunner`. The command sandbox uses a digest-pinned image, read-only root filesystem, non-root identity, all-capability drop, no-new-privileges, isolated namespaces, bounded CPU/RAM/PID/tmpfs/lifetime, and `--network none`.

The current Draft implementation has a partial hold boundary. After successful `podman create` yields an admitted exact container ID, the runtime calls `podman init <id>`, fails closed and cleans up that exact ID if init fails, validates static/configuration evidence while the container remains held, then calls `podman start <id>` and retains the existing live effective-process verifier. The pre-start verifier rejects configured state that already disproves the P0 profile; it is deliberately not labeled effective seccomp/LSM/capability proof.

That sequence does **not** close issue #25. Final seccomp/LSM/capability state can be established at or near process exec, while `podman init` and static inspection occur before the consumer process exists. The accepted direction therefore remains a two-phase pre-payload boundary that holds consumer execution through effective attestation: establish the sandbox, obtain the required effective isolation evidence in the relevant process context, evaluate it fail closed, and only then release the consumer process.

A supported OCI lifecycle primitive such as `startContainer`, or an equivalent backend mechanism, may be used only if the runtime proves the following properties through RED-driven tests and real backend acceptance:

1. the consumer process cannot become runnable before positive attestation;
2. attestation code is runtime-owned rather than supplied by the hostile image;
3. the attestor is immutably identified and architecture-compatible;
4. any attestor mount, control channel and evidence channel are separately bounded and included in the isolation proof;
5. the backend capability is detected explicitly and absence/malformed evidence fails closed;
6. applied mount/resource/namespace checks and live seccomp/LSM/capability evidence are obtained before consumer release;
7. the exact acquired container identity remains the sole post-create lifecycle/destructive authority; and
8. real rootless-Podman E2E on an eligible LSM backend independently proves the effective boundary.

Podman's `--hooks-dir` can inject OCI hooks, but hook support alone is not acceptance. OCI requires a `startContainer` hook path to resolve in the container namespace, so an arbitrary hostile tool image cannot be assumed to contain a trusted attestor. Any hook-based implementation must deliver the runtime-owned attestor through an explicitly reviewed narrow path rather than trusting image content.

Completion remains observed with bounded `podman wait`; bounded output is collected with `podman logs` using a retaining log driver. A nonzero workload exit code remains a structured workload result. Administrative Podman timeout, output-bound overflow, invocation failure, malformed evidence, or cleanup failure remains a runtime error.

Every post-create failure path must attempt cleanup against the acquired exact container identity. If cleanup itself fails, that failure is explicit evidence and must not be hidden behind an earlier isolation/log/wait error.

### Transport

Add the synchronous `quarantine-sandbox-runtime run` CLI using direct argv after `--`. The CLI validates against an operator policy ceiling, invokes the production backend, prints a structured JSON result on success, and does not add an HTTP listener or shell-string parser.

An optional PR-source input is accepted only as a complete tuple: an absolute host path to a trusted caller's materialized tree, an exact lower-case Git SHA-1/SHA-256 revision, and the expected canonical tree SHA-256. The runtime never mounts that path directly. It copies only regular files into a bounded temporary tree, rejects links and special files, strips every executable bit, verifies the sorted path-and-content manifest digest, and mounts only the verified staging tree at `/workspace` with `ro,noexec,nosuid,nodev`. Container inspection must confirm those mount controls before the result can carry the exact-revision receipt. The receipt binds the asserted revision to the verified tree digest, file/byte totals, and executable-bit removal count; it does not claim that the runtime fetched or authorized the revision.

## Alternatives

- **Static-inspect fallback or static inspection alone before `start`:** rejected because configured/applied state is not effective-runtime proof and does not establish the consumer process's actual confinement. Pre-start static verification remains useful only as an early rejection gate.
- **Treat `podman init` plus static inspection as complete issue #25 GREEN:** rejected. `init` holds the payload and permits earlier rejection, but it does not by itself prove the effective seccomp/LSM/capability state that will apply to the consumer process at exec.
- **Start an inert container and later invoke the consumer with ordinary `podman exec`:** rejected as insufficient because the same pre-attestation race moves to the exec process unless an equivalent pre-exec attestation/release boundary is proven.
- **Use `createRuntime` or `createContainer` alone as the attestation point:** rejected because OCI does not guarantee cgroups and SELinux/AppArmor are already applied at those stages.
- **Treat `startContainer` hook support as sufficient without a runtime-owned attestor:** rejected because the hook path resolves in the container namespace and hostile image content cannot become the security authority.
- **First transport as an HTTP service:** rejected because the current contract is one-shot and does not require listener/lifecycle semantics.
- **Attach directly to workload pipes as the sole completion mechanism:** rejected because `podman wait` provides a clearer authoritative container exit-code boundary for this detached-verification flow.
- **Consumer-owned Podman calls:** rejected because that would duplicate isolation policy and backend-specific security logic outside the canonical runtime.
- **Mount the consumer checkout directly:** rejected because mutable files, links, executable modes, and path races would cross the host/runtime trust boundary without a verified immutable staging identity.

## Verification rule

The executed `podman init` capability RED and its minimum production repair must remain GREEN on the current owner lineage: init unavailability must fail closed before start, and configuration that already contradicts the P0 profile must be rejected while the exact acquired container remains held. Those checks are necessary but not sufficient for issue #25.

Before production GREEN for the full P0, a checked-in effective hold/attest/release capability RED must go beyond the init/static gate and prove that the selected Podman/OCI mechanism can keep consumer code non-runnable while runtime-owned attestation executes in the required effective context, exposes bounded authoritative evidence, and releases only after positive policy evaluation. The original hostile payload-side-effect RED must then become GREEN on the same implementation.

Release claims require all of the following on one immutable integrated source identity: exact-head unit/property/coverage evidence; real rootless-Podman command E2E; positive effective LSM/seccomp/capability/resource/network proof on an eligible backend; hostile negative fixtures; cleanup leak rejection; package/SBOM/provenance/reproducibility evidence; and current review/governance gates. Queued, skipped, predecessor, static-only, fake-backend-only, or locally reported evidence is not release authority.

## Traceability

See `docs/doctoring/COMMAND_PRE_ATTESTATION_TRACEABILITY.md` for the exact #25 RED, the executed init-hold RED/minimum partial repair, owner invariant, alternatives, next effective-attestation RED, and APA 7th references to the OCI Runtime Specification, Podman lifecycle/hook documentation, and NIST SP 800-190.