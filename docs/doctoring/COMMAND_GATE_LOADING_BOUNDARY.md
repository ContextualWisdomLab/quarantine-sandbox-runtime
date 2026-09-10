# Runtime gate ELF loading boundary

Status: **Proposed / loader admission repair implemented / exact-head verification pending**. This is a focused supplement to ADR-0008 and issue #25, not a new runtime, a released contract, or proof that QSR confinement is complete.

## Scope and authority

Canonical parent/base is PR #14 exact `645e05825d9991f3054a69efb9dd4231a55e95ce` (`feat/podman-command-execution-backend`). This child remains an ordinary, non-force descendant of that exact parent and retains the parent hold/attest/release work. Protected integration and release authority remain with the canonical owner stack.

The child began as a path-isolated test-first loader-boundary PR. Once the Rust test target reached the Test step, the owner repair necessarily extended into `RuntimeGateArtifact::stage` and into positive staging fixtures whose former use of a dynamically linked `std::env::current_exe()` contradicted the corrected admission contract. No workflow, dependency, credential, AGENTS/CLAUDE, protected-branch, or consumer-source mutation is introduced.

## Finding and counterevidence

The parent `src/infrastructure/runtime_gate_artifact.rs` authenticated the expected SHA-256 and ELF `e_machine`, then staged the bytes. `executable_architecture()` inspected only magic, byte order and machine in the first 20 bytes. Neither staging nor that parser validated the ELF64 program-header table or rejected `PT_INTERP`. A matching hash of the gate therefore did not authenticate an interpreter selected from the workload image.

Linux uses the interpreter named by `PT_INTERP` when starting a dynamically linked ELF file. Image-owned startup code can consequently run before the gate reaches `main()`, announces READY, or reads its release token. A gate-file digest remains necessary provenance, but it is not sufficient evidence of image-independent startup.

Counterevidence is preserved: `tests/podman_hold_gate_capability.rs` already builds its real held-gate fixture for `x86_64-unknown-linux-musl`. This finding concerns admission enforcement. It is not a claim that the existing static-musl capability experiment used a compromised loader, that a released QSR deployment was exploited, or that a container escape occurred.

## Executed Linux mechanism witness

`tests/fixtures/runtime_gate_elf_loader_probe.mjs` uses Node built-ins plus local compiler/binutils to compare a held main program with an otherwise identical executable whose `PT_INTERP` names a harmless replacement loader. On Linux x86_64, kernel 6.18.35, GCC 14.2.0 and Node 22.16.0, two fresh builds completed three matched trials each. All six comparisons preserved the gate SHA-256, supplied no release token, observed the replacement interpreter marker before any gate-main marker, and preserved the self-contained static control reaching main and rejecting EOF with exit 77.

This is OS-loader evidence, not Podman-confinement evidence and not a measured attack-prevention rate.

## Rust RED and repair

Five Linux x86_64/aarch64 tests in `tests/command_gate_loading_boundary_red.rs` call the exported `RuntimeGateArtifact::stage` API with data-only ELF64 fixtures. They require two positive admissions—self-contained ET_EXEC and ET_DYN/static PIE—and three fail-closed cases: `PT_INTERP`, a truncated program-header table, and an overflowing program-header offset.

Original runs were blocked by rustfmt. Formatter-only exact `2566653126e5a5b0a14e56e467b9e6b84452485d`, native CI `34430312729`, passed exact checkout, dependency lock, repository policy, coverage-parser tests and rustfmt, then verify `102724264748` failed in the Rust Test step; coverage `102724265011` likewise reached production-test execution and failed. The available GitHub connector does not expose the job stdout, so this record does not invent which individual assertion produced that Test-step failure. The source-level counterexample and six-run OS loader witness remain the causal basis for the repair.

The minimum production repair is implemented on the current child lineage. `RuntimeGateArtifact::stage` now accepts only a bounded ELF64 ET_EXEC or ET_DYN loading profile with a valid ELF version/header shape, a representable complete fixed-size program-header table, at least one `PT_LOAD`, and no `PT_INTERP`. Expected SHA-256 and host/ELF machine checks remain separate prerequisites. Rejection is typed as `RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary`.

The previous positive staging fixtures that used the dynamically linked Rust test harness were not grandfathered. They were replaced with data-only self-contained ELF fixtures before enabling the stricter production admission. This preserves the intended digest, architecture, clone-lifetime, remapped-user permission, binding-plan and bounded release-control assertions without normalizing a dynamic interpreter into trusted gate authority. The shared integration-test fixture lives under `tests/support/runtime_gate_fixture.rs`; no fixture bytes are executed.

Current child source head before this documentation commit is `c21745ff85cb70911b82c54c84bcd71bfdbf4b2d`. Native CI `34432397754` was created for that exact source candidate; at the last read all five jobs were queued, so no exact-head GREEN is claimed here. The documentation commit itself requires fresh exact-head verification and supersedes predecessor CI as merge authority.

## Acceptance boundary

The focused loader target must show zero unsafe admissions across the three negative cases and both positive controls admitted on the exact repaired source. Full fmt/tests/Clippy/rustdoc and owned-production statement/function/region/branch coverage must then be reacquired. The real hostile-loader witness and real rootless held-gate E2E remain independent evidence classes, and a dedicated positive effective-LSM lane remains mandatory.

This loader repair still does not close issue #25. Canonical `RootlessPodmanAdapter::run_command_at` on PR #14 continues to make hostile consumer argv the OCI entrypoint and verifies live effective isolation only after `podman start`. After this child is exact-head GREEN, its complete delta must be adopted by ordinary/non-force integration into #14, followed by reacquisition of the parent hold → effective attestation → bounded one-time release → trusted pre-exec ACK → exact consumer exec lifecycle tests.

No protected merge, version/tag/package/GitHub Release, immutable consumer publication, or Agent-containment claim is authorized by this child alone.

## Primary references (APA 7)

Linux man-pages project. (n.d.). *execve(2) — Linux manual page*. https://man7.org/linux/man-pages/man2/execve.2.html

Xinuos. (n.d.). *ELF Object File Format: Program header*. https://gabi.xinuos.com/elf/07-pheader.html

Xinuos. (n.d.). *ELF Object File Format: Program interpreter*. https://gabi.xinuos.com/elf/09-dynamic.html
