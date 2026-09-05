# ADR 0008: Podman-backed command execution and a CLI transport

- **Status:** Proposed
- **Date:** 2026-09-02

This ADR remains Proposed while PR #14 is Draft. Acceptance requires protected integration of the prerequisite stack, fresh exact-head execution of the current cleanup regressions, effective-isolation proof on an eligible LSM-capable backend, and release-grade evidence under live governance.

## Context

ADR-0007 defines a bounded run-to-completion contract but intentionally ships no production backend or transport. The next slice needs a real rootless-Podman implementation that reuses the existing isolation policy and process supervisor, plus a minimal transport usable by CI/security consumers.

A security review on this candidate corrected an earlier design idea: static `podman container inspect` configuration is evidence of requested/configured state, not positive proof of effective per-process seccomp/LSM/capability enforcement. `run_command_at` therefore fails closed when live process evidence cannot be obtained; it does not fall back to static-only attestation. Fast-exiting workloads remain a compatibility gap to solve with a stronger verified execution/attestation handshake rather than weaker evidence.

## Decision

### Backend

`RootlessPodmanAdapter::run_command_at` implements `CommandExecutionBackend` and reuses the existing P0 isolation policy and `BoundedCommandRunner`. The command sandbox uses a digest-pinned image, read-only root filesystem, non-root identity, all-capability drop, no-new-privileges, isolated namespaces, bounded CPU/RAM/PID/tmpfs/lifetime, and `--network none`.

The adapter starts the container, then verifies effective isolation from container inspection plus live `podman top` process evidence before trusting the sandbox. The verified process evidence includes seccomp mode, LSM label/domain, and effective/bounding/inheritable/permitted/ambient capability sets. Missing, malformed, contradictory, `unconfined`, AppArmor complain-mode, or otherwise insufficient evidence fails closed.

Completion is observed with bounded `podman wait`; bounded output is collected with `podman logs` using a retaining log driver. A nonzero workload exit code remains a structured workload result. Administrative Podman timeout, output-bound overflow, invocation failure, malformed evidence, or cleanup failure remains a runtime error.

Every post-create failure path must attempt cleanup. If cleanup itself fails, that failure is explicit evidence and must not be hidden behind an earlier isolation/log/wait error. Current test-only REDs in PR #14 pin the remaining cleanup branches before production generalization.

### Transport

Add the synchronous `quarantine-sandbox-runtime run` CLI using direct argv after `--`. The CLI validates against an operator policy ceiling, invokes the production backend, prints a structured JSON result on success, and does not add an HTTP listener or shell-string parser.

An optional PR-source input is accepted only as a complete tuple: an absolute host path to a trusted caller's materialized tree, an exact lower-case Git SHA-1/SHA-256 revision, and the expected canonical tree SHA-256. The runtime never mounts that path directly. It copies only regular files into a bounded temporary tree, rejects links and special files, strips every executable bit, verifies the sorted path-and-content manifest digest, and mounts only the verified staging tree at `/workspace` with `ro,noexec,nosuid,nodev`. Container inspection must confirm those mount controls before the result can carry the exact-revision receipt. The receipt binds the asserted revision to the verified tree digest, file/byte totals, and executable-bit removal count; it does not claim that the runtime fetched or authorized the revision.

## Alternatives

- **Static-inspect fallback when the process exits before live attestation:** rejected because configured state is not effective-runtime proof. Future support for extremely short commands must preserve positive attestation, for example with a reviewed execution/hold handshake, rather than silently weakening the control.
- **First transport as an HTTP service:** rejected because the current contract is one-shot and does not require listener/lifecycle semantics.
- **Attach directly to workload pipes as the sole completion mechanism:** rejected because `podman wait` provides a clearer authoritative container exit-code boundary for this detached-verification flow.
- **Consumer-owned Podman calls:** rejected because that would duplicate isolation policy and backend-specific security logic outside the canonical runtime.
- **Mount the consumer checkout directly:** rejected because mutable files, links, executable modes, and path races would cross the host/runtime trust boundary without a verified immutable staging identity.

## Verification rule

Release claims require all of the following on one immutable integrated source identity: exact-head unit/property/coverage evidence; real rootless-Podman command E2E; positive effective LSM/seccomp/capability/resource/network proof on an eligible backend; hostile negative fixtures; cleanup leak rejection; package/SBOM/provenance/reproducibility evidence; and current review/governance gates. Queued, skipped, predecessor, static-only, or locally reported evidence is not release authority.
