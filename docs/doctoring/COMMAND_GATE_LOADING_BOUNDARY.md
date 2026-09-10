# Runtime gate ELF loading boundary

Status: **Proposed / regression tests pending Rust execution**. This is a focused
supplement to ADR-0008 and issue #25, not a new runtime, a released contract, or
proof that QSR confinement is complete.

## Scope and authority

Inspected parent: PR #14, `77cef4fda661b3e65c602a661b9c5a6db0c84295`.
Parent tree: `d8afa7f51cfc52ebb5db8138e836a1bfd8e9d9c3`.
The parent base is `00bd3654d5f1a58cf6d1e23bded1986156645ba0`; protected
`develop` remains `60a85c7633e03b425b67159ec6822c8178cf87ea` at inspection.

This child adds only `tests/command_gate_loading_boundary_red.rs`,
`tests/fixtures/runtime_gate_elf_loader_probe.mjs`, and this traceability record.
It does not move the active parent branch, alter its stdin RED, mutate
`src/`, change dependencies/workflows, or copy QSR into Noema/CO. Shared
AGENTS/CLAUDE/ADR/Gap-ledger changes stay with the coordinating parent writer.

## Finding and counterevidence

`src/infrastructure/runtime_gate_artifact.rs` authenticates the expected digest
and ELF `e_machine`, then stages the bytes. `executable_architecture()` inspects
only magic, byte order and machine in the first 20 bytes. Neither the staging
path nor this parser validates the ELF program-header table or rejects
`PT_INTERP`. A matching hash of the gate does not authenticate the interpreter
and shared libraries resolved from a workload image.

Linux uses the interpreter named by `PT_INTERP` when starting a dynamically
linked ELF file. Consequently image-owned startup code can run before the gate
reaches `main()`, announces READY, or reads its release token. No LLM decision,
shell command approval, or successful gate-file hash comparison closes that
startup dependency boundary.

Important counterevidence: `tests/podman_hold_gate_capability.rs` already builds
its actual held-gate fixture for `x86_64-unknown-linux-musl`. That is the right
direction. This finding concerns **admission enforcement**, not an assertion
that the existing static-musl capability experiment used a compromised loader,
that a released QSR deployment is exploited, or that a container escape occurred.

## Executed Linux mechanism witness

Command (Node built-ins only; requires local cc and readelf):

```sh
node tests/fixtures/runtime_gate_elf_loader_probe.mjs
```

On Linux x86_64, kernel 6.18.35, GCC 14.2.0 and Node 22.16.0, two fresh local
builds each completed three matched trials. Each trial:

- executed a dynamically linked held-main fixture with the ordinary loader;
- replaced only its temporary interpreter with a harmless fixed-marker ELF;
- verified the gate file's SHA-256 was unchanged and sent no release token;
- observed the replacement interpreter's marker before any gate-main marker;
- ran the self-contained static control, which still reached held main and
  rejected EOF with exit 77.

All six matched comparisons met those assertions. Temporary binaries were
removed by the harness. No secrets, networking, privileged operations, runtime
sockets, or host-system files were changed. The embedded temporary interpreter
path varies across independent builds; hash equality is asserted **within**
each comparison, not across builds.

This is a real OS-loading witness using small C/assembly fixtures. It is **not**
execution of `RuntimeGateArtifact::stage`, the Rust regression suite, or Podman
confinement. Do not count it as QSR RED/GREEN or a measured attack-prevention rate.

## Rust acceptance tests

The new test target calls the real `RuntimeGateArtifact::stage` API with
self-authored, digest-matching ELF64 files. It never executes fixture bytes.
Two positive controls preserve a self-contained ET_EXEC and a self-contained
ET_DYN/static-PIE file. Three negatives require rejection of an image-owned
interpreter, a truncated program-header table, and an overflowing table offset.
The two x86_64 positive fixture layouts were independently checked with readelf
and executed as harmless exit-77 files. AArch64 fixture encoding has not been
executed on an AArch64 host.

```sh
cargo test --locked --test command_gate_loading_boundary_red -- --nocapture
```

Rust/Cargo/Podman were unavailable in the local review environment, and direct
compiler downloads failed DNS resolution. The five Rust tests have **not been
compiled or executed** here. The predicted baseline is two accepted positive
controls and three rejected-by-test admissions; it is a source-derived
prediction, not a reported test result. A missing compiler, setup failure,
queued job or an earlier unrelated test failure does not establish causal RED.

## Minimum owner repair after causal RED

In QSR infrastructure, enforce a bounded, structurally valid, image-independent
loading profile before staging a gate. For the selected P0 profile reject
`PT_INTERP` and external loader dependencies; preserve legitimately
self-contained static PIE rather than rejecting every ET_DYN file. A binary
hash, `musl` target name, or absence of a textual path match is not sufficient:
parse the actual bounded ELF structures and verify the built artifact. Keep
release provenance and a trusted expected digest as separate requirements.

The existing positive staging fixtures based on `std::env::current_exe()` must
be reconciled when that executable is dynamically linked. Replace them with
valid self-contained fixtures; do not disable the new negative or widen the
profile to make old fixture assumptions pass. After focused RED/GREEN, verify
full tests, fmt, Clippy, rustdoc and the unchanged production coverage gates.

Real acceptance additionally requires a QSR/Podman experiment with a hostile
image loader/library and zero image-code side effects before effective
attestation and release. Absence of PT_INTERP alone is not complete sandbox
safety. Gate artifact TOCTOU, inherited descriptors/environment, lifecycle
integration, acquired-ID cleanup, runtime/LSM enforcement, output authority and
published consumer contracts retain their own acceptance criteria.

## KPI and handoff

For the focused Rust target, record executed/attempted tests, setup failures,
unsafe admissions per three negative cases, and accepted positives per two
positive cases. Targets are zero unsafe admissions and two accepted controls;
current Rust baseline is **unmeasured**, not zero. Log exact source SHA and
platform separately for each run. Local mechanism comparisons are 6/6 across
two builds and stay in their own evidence class.

The active #14 writer retains production ownership. Integrate this child only
by ordinary, non-force preservation of its tests and evidence; do not import a
mutable child into consumers. Do not close #25, mark #14 Ready, merge, publish,
or advertise Agent containment based on this supplement. Consumer port/ACL and
no-host-fallback tests can proceed independently while the runtime is repaired.

## Primary reference (APA 7)

Linux man-pages project. (n.d.). *execve(2) — Linux manual page*.
https://man7.org/linux/man-pages/man2/execve.2.html

Relevant contract: DESCRIPTION, dynamically linked ELF and PT_INTERP. The
source inspection above concerns the exact GitHub parent, not the manual.
