# ADR 0008: Podman-backed command execution and a CLI transport

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while PR #14 is Draft. Acceptance requires protected integration of the prerequisite stack, a pre-payload effective-isolation boundary that makes issue #25 GREEN, fresh exact-head execution of the current cleanup and isolation regressions, effective-isolation proof on an eligible LSM-capable backend, and release-grade evidence under live governance.

## Context

ADR-0007 defines a bounded run-to-completion contract but intentionally ships no production backend or transport. The next slice needs a real rootless-Podman implementation that reuses the existing isolation policy and process supervisor, plus a minimal transport usable by CI/security consumers.

A security review corrected an earlier design idea: static `podman container inspect` configuration is evidence of requested/configured state, not positive proof of effective per-process seccomp/LSM/capability enforcement. `run_command_at` therefore fails closed when live process evidence cannot be obtained; it does not fall back to static-only attestation.

Issue #25 exposed a stronger lifecycle defect. Exact PR #14 head `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, executes `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` and fails because the consumer command is already the OCI process when `podman start` is invoked. A later live-attestation error and cleanup cannot undo hostile code that became runnable before that evidence was sampled. The previous prose saying the runtime inspected the running process before allowing the requested command to execute was therefore incorrect and is superseded by this Proposed decision.

OCI runtime lifecycle semantics provide a candidate ordering primitive, not a ready-made product implementation. OCI `create` establishes the runtime environment without running the user-specified program. During `start`, `startContainer` hooks execute before the user-specified process. By contrast, OCI explicitly notes that `createRuntime` and `createContainer` hooks may run before cgroups and SELinux/AppArmor setup is complete. A production pre-payload boundary must preserve this distinction.

## Decision

### Backend

`RootlessPodmanAdapter::run_command_at` implements `CommandExecutionBackend` and reuses the existing P0 isolation policy and `BoundedCommandRunner`. The command sandbox uses a digest-pinned image, read-only root filesystem, non-root identity, all-capability drop, no-new-privileges, isolated namespaces, bounded CPU/RAM/PID/tmpfs/lifetime, and `--network none`.

The current Draft implementation starts the consumer OCI process and only then verifies container inspection plus live process evidence. That ordering is intentionally treated as RED under issue #25 and is not an accepted execution architecture.

The accepted direction is a two-phase pre-payload boundary: establish the sandbox, hold the consumer process, obtain the required effective isolation evidence, evaluate it fail closed, and only then release the consumer process. The implementation may use a supported OCI lifecycle primitive such as `startContainer`, or an equivalent backend mechanism, only if the runtime proves the following properties through RED-driven tests and real backend acceptance:

1. the consumer process cannot become runnable before positive attestation;
2. attestation code is runtime-owned rather than supplied by the hostile image;
3. the attestor is immutably identified and architecture-compatible;
4. any attestor mount, control channel and evidence channel are separately bounded and included in the isolation proof;
5. the backend capability is detected explicitly and absence/malformed evidence fails closed;
6. applied mount/resource/namespace checks and live seccomp/LSM/capability evidence are obtained before consumer release;
7. the exact acquired container identity remains the sole post-create lifecycle/destructive authority; and
8. real rootless-Podman E2E on an eligible LSM backend independently proves the effective boundary.

Podman's `--hooks-dir` can inject OCI hooks, but hook support alone is not acceptance. OCI requires a `startContainer` hook path to resolve in the container namespace, so an arbitrary hostile image cannot be assumed to contain a trusted attestor. Any hook-based implementation must deliver the runtime-owned attestor through an explicitly reviewed narrow path rather than trusting image content.

Completion remains observed with bounded `podman wait`; bounded output is collected with `podman logs` using a retaining log driver. A nonzero workload exit code remains a structured workload result. Administrative Podman timeout, output-bound overflow, invocation failure, malformed evidence, or cleanup failure remains a runtime error.

Every post-create failure path must attempt cleanup against the acquired exact container identity. If cleanup itself fails, that failure is explicit evidence and must not be hidden behind an earlier isolation/log/wait error.

### Transport

Add the synchronous `quarantine-sandbox-runtime run` CLI using direct argv after `--`. The CLI validates against an operator policy ceiling, invokes the production backend, prints a structured JSON result on success, and does not add an HTTP listener or shell-string parser.

An optional PR-source input is accepted only as a complete tuple: an absolute host path to a trusted caller's materialized tree, an exact lower-case Git SHA-1/SHA-256 revision, and the expected canonical tree SHA-256. The runtime never mounts that path directly. It copies only regular files into a bounded temporary tree, rejects links and special files, strips every executable bit, verifies the sorted path-and-content manifest digest, and mounts only the verified staging tree at `/workspace` with `ro,noexec,nosuid,nodev`. Container inspection must confirm those mount controls before the result can carry the exact-revision receipt. The receipt binds the asserted revision to the verified tree digest, file/byte totals, and executable-bit removal count; it does not claim that the runtime fetched or authorized the revision.

## Alternatives

- **Static-inspect fallback or reordering static inspection before `start`:** rejected because configured/applied state is not effective-runtime proof and does not establish the consumer process's actual confinement.
- **Start an inert container and later invoke the consumer with ordinary `podman exec`:** rejected as insufficient because the same pre-attestation race moves to the exec process unless an equivalent pre-exec attestation/release boundary is proven.
- **Use `createRuntime` or `createContainer` alone as the attestation point:** rejected because OCI does not guarantee cgroups and SELinux/AppArmor are already applied at those stages.
- **Treat `startContainer` hook support as sufficient without a runtime-owned attestor:** rejected because the hook path resolves in the container namespace and hostile image content cannot become the security authority.
- **First transport as an HTTP service:** rejected because the current contract is one-shot and does not require listener/lifecycle semantics.
- **Attach directly to workload pipes as the sole completion mechanism:** rejected because `podman wait` provides a clearer authoritative container exit-code boundary for this detached-verification flow.
- **Consumer-owned Podman calls:** rejected because that would duplicate isolation policy and backend-specific security logic outside the canonical runtime.
- **Mount the consumer checkout directly:** rejected because mutable files, links, executable modes, and path races would cross the host/runtime trust boundary without a verified immutable staging identity.

## Verification rule

Before production GREEN for issue #25, a checked-in backend-capability RED must prove that the selected Podman/OCI mechanism can hold consumer execution, invoke a runtime-owned attestor in the required effective context, expose bounded authoritative evidence, and release only after positive policy evaluation. The existing payload-side-effect RED must then become GREEN on the same implementation.

Release claims require all of the following on one immutable integrated source identity: exact-head unit/property/coverage evidence; real rootless-Podman command E2E; positive effective LSM/seccomp/capability/resource/network proof on an eligible backend; hostile negative fixtures; cleanup leak rejection; package/SBOM/provenance/reproducibility evidence; and current review/governance gates. Queued, skipped, predecessor, static-only, fake-backend-only, or locally reported evidence is not release authority.

## Traceability

See `docs/doctoring/COMMAND_PRE_ATTESTATION_TRACEABILITY.md` for the exact #25 RED, owner invariant, alternatives, next capability RED, and APA 7th references to the OCI Runtime Specification, Podman hook documentation, and NIST SP 800-190.
