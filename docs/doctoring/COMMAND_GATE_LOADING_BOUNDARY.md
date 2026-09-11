# Runtime gate ELF loading boundary

Status: **Implemented on the canonical command/runtime lineage; protected integration and release evidence remain pending**. This is a focused supplement to Proposed ADR-0008 and issue #25, not a new runtime or proof that QSR confinement is complete.

## Scope and authority

The runtime gate is owned by canonical PR #14 (`feat/podman-command-execution-backend`). The canonical command path retains the hold/attest/release composition and treats the gate bytes as host-owned immutable release material. No workflow, credential, consumer-source, or mutable sibling dependency is introduced by this loading-boundary repair.

The loader work began as a path-isolated test-first finding and was later adopted into #14 by ordinary ancestry. Positive fixtures that used a dynamically linked Rust test executable were replaced with data-only self-contained ELF fixtures rather than grandfathering image-owned interpreter behavior into the trusted gate boundary.

## Finding and counterevidence

The original `RuntimeGateArtifact::stage` authenticated expected SHA-256 and ELF `e_machine`, then staged the bytes. Its architecture parser inspected only magic, byte order and machine in the first 20 bytes. It did not prove the ELF64 program-header table or reject `PT_INTERP`. A matching hash of the gate file therefore did not by itself prove that startup would remain independent from code selected from the workload image.

Linux uses the interpreter named by `PT_INTERP` when starting a dynamically linked ELF file. Image-owned startup code could consequently execute before a gate program reached `main()`, announced readiness, or read a release token. Gate-file digest remains necessary provenance, but it is not sufficient loading-boundary evidence.

Counterevidence is preserved: the real held-gate capability fixture is built for `x86_64-unknown-linux-musl`. This finding concerns admission enforcement; it is not a claim that the static-musl capability experiment used a compromised loader, that a released deployment was exploited, or that a container escape occurred.

## Executed Linux mechanism witness

`tests/fixtures/runtime_gate_elf_loader_probe.mjs` uses local compiler/binutils tooling to compare a held main program with an otherwise identical executable whose `PT_INTERP` names a harmless replacement loader. Historical Linux x86_64 trials preserved the gate SHA-256, supplied no release token, observed the replacement-interpreter marker before any gate-main marker, and preserved the self-contained static control reaching main and rejecting EOF with exit 77.

This is OS-loader evidence, not Podman-confinement evidence and not a measured attack-prevention rate.

## Rust RED, repair, and loading taxonomy

`tests/command_gate_loading_boundary_red.rs` exercises the exported `RuntimeGateArtifact::stage` boundary with data-only ELF64 fixtures. Positive controls admit self-contained ET_EXEC and ET_DYN/static PIE. Negative cases reject `PT_INTERP`, ELF32, invalid identification version, unsupported object type, malformed header sizes, zero or overlapping program-header tables, absence of `PT_LOAD`, truncated/overflowing tables, and unsupported ELF data encoding.

The first loader implementation correctly failed closed for unsupported `EI_DATA`, but it classified the public staging request as `ArchitectureMismatch`: `stage()` interpreted machine identity before the complete self-contained loading validator ran. That ordering also left the validator's unsupported-data-encoding rejection unreachable through the public admission API.

Exact test-first commit `e9d111b8b53ca9454c28f0c1f36597dea3e81455` changed the public regression to require `RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary` for `EI_DATA=0`. Native CI `34525809193`, verify `103034299135`, reached the Rust Test step and failed, while hosted negative rootless/AppArmor `103034299151` remained GREEN. This is the causal RED for the taxonomy/ordering defect.

Minimum production commit `9b86116d02e719e10ed04de0616877d0575d4323` moves `validate_self_contained_elf_loading(&bytes)` ahead of `executable_architecture(&bytes)`. Digest identity is still checked first; configured architecture must still equal the host before source admission; and valid self-contained ELF bytes still must match the expected machine architecture. The change only prevents machine fields from determining the public rejection taxonomy before ELF class/data/version/program-header safety is established.

Exact native CI `34526012470` on `9b86116d...` proved the repaired test GREEN and then passed Clippy and rustdoc in verify `103034975615`. Hosted negative rootless/AppArmor `103034975643` was also GREEN, including real held-gate capability, unavailable effective-LSM fail-closed cleanup, and runtime-owned leak rejection. A later descendant supersedes that source head as merge authority, so predecessor GREEN remains causal repair evidence rather than current release authority.

This ordering matches the ELF contract rather than inventing a QSR-specific interpretation: `e_ident[EI_DATA]` defines how multi-byte object fields are encoded, `ELFDATANONE` (`0`) is an invalid data encoding, and only defined encodings may be used to interpret fields such as `e_machine`. Accordingly an undefined byte order is first a malformed executable-loading boundary, not evidence of a different CPU architecture.

## Host byte-order compatibility follow-on

A second review showed that defined encoding is necessary but not sufficient for a **trusted executable on this runtime target**. The bounded parser intentionally understands both `ELFDATA2LSB` and `ELFDATA2MSB`, and `executable_architecture` can therefore decode a valid `e_machine` from either representation. Before the follow-on repair, a structurally valid, digest-matched big-endian ELF could collapse to the same `x86_64` or `aarch64` architecture string and be staged on a little-endian compilation target.

Test-only exact `0f56cbcf61fede7ba18156319021b3813dacc910` rebuilt the self-contained fixture as a structurally valid big-endian ELF64, recomputed its SHA-256, and required public staging to fail closed as `UnsafeExecutableLoadingBoundary`. Native CI `34545856385`, verify `103098180914`, executed the full workspace/all-target/no-fail-fast test step and failed specifically because that fixture was admitted. This is the causal RED for host-byte-order compatibility; it is not inferred from a parser-only unit test.

Minimum ordinary descendant `a9ebb2b800018d3f02b0c118b948a1d506a2a07d` retains endian-aware bounded parsing but adds a separate trusted-admission condition: ELF `EI_DATA` must match Rust's compile-time `target_endian` before machine identity is accepted and bytes are staged. The implementation uses `#[cfg(target_endian = "little")]` / `#[cfg(target_endian = "big")]` to define the accepted `EI_DATA`. The Rust Reference defines `target_endian` as exactly the target CPU's `little` or `big` byte order; the ELF gABI defines `ELFDATA2LSB` and `ELFDATA2MSB` as the file's basic-object encoding. The selected contract therefore keeps the parser general while binding executable authority to the target that actually runs the gate.

Exact `a9ebb2b8...` CI `34549771316` subsequently made verify and hosted negative rootless/AppArmor GREEN; coverage evidence was generated and failed only the repository's explicit 100% admission. Test-only `9dbb359f1b5c1d60b41cca721698b646298bee59` then exercised the real regular-file `fs::read` -> `SourceReadFailed` boundary with Linux `/proc/self/mem`; its verify `103121141805` and hosted negative `103121141640` completed GREEN before the branch advanced. Current workflow-only descendant `dad66fef7f95199000ce38de2a6ab1ce07f3ab54` also has exact verify `103123694946` and hosted negative `103123695099` GREEN; its uploaded coverage evidence remains below the required 100% gate, and dedicated positive SELinux still lacks an eligible runner. These later runs strengthen regression evidence but do not turn this focused loader slice into release authority.

## Acceptance boundary

The loader target must retain zero unsafe admissions across malformed/self-interpreted negative cases and must accept only a self-contained ELF whose declared byte order is compatible with the compilation target. Endian-aware parsing of defined foreign encodings remains useful for deterministic rejection and architecture diagnostics; it is **not** a public positive executable-admission promise. Full fmt/tests/Clippy/rustdoc and owned-production statement/function/region/branch coverage must be reacquired on every new exact head. Real hostile-loader mechanism evidence, real rootless held-gate E2E, and dedicated positive effective-LSM evidence remain independent evidence classes.

This loader repair does not close issue #25. Canonical command execution now composes the verified immutable gate as OCI PID 1, performs configured contradiction checks and live effective isolation attestation before bounded one-time release, requires trusted pre-exec acknowledgement, detaches release stdin, executes exact consumer argv, and cleans up by exact acquired ID. Issue #25 remains the release-evidence envelope until one unchanged integrated candidate satisfies coverage, positive-LSM, review/security, protected integration, immutable publication, SBOM/provenance/reproducibility and rollback gates.

No protected merge, version/tag/package/GitHub Release, immutable consumer publication, or Agent-containment claim is authorized by this loader slice alone.

## Primary references (APA 7)

Linux Foundation. (n.d.). *ELF: Executable and Linkable Format*. https://refspecs.linuxfoundation.org/elf/elfspec.pdf

Linux man-pages project. (n.d.). *execve(2) — Linux manual page*. https://man7.org/linux/man-pages/man2/execve.2.html

The Rust Project Developers. (n.d.). *Conditional compilation: `target_endian`*. The Rust Reference. https://doc.rust-lang.org/reference/conditional-compilation.html#target_endian

Xinuos. (2025). *ELF Object File Format* (Version 4.3 DRAFT, 2025-10-20), §2.3 Data Encoding. https://gabi.xinuos.com/elf.pdf

Xinuos. (n.d.). *ELF Object File Format: Program header*. https://gabi.xinuos.com/elf/07-pheader.html

Xinuos. (n.d.). *ELF Object File Format: Program interpreter*. https://gabi.xinuos.com/elf/09-dynamic.html
