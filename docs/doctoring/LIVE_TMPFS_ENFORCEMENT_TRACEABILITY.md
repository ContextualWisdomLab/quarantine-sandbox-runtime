# Live `/tmp` enforcement traceability

## Scope

This record tracks the real-runtime acceptance required by issue #35 for the command/application-service rootless Podman boundary. Configured `Tmpfs`, `HostConfig`, or create-command evidence is not sufficient for release acceptance; the reviewed candidate must observe the effective mount from inside the running sandbox.

## Current candidate

- Owner PR: #112 (`automation/command-chronology-owner-repair`)
- Source candidate introducing the live witness: `01e5b9470cb116b116c585779d86fc40adccf1e3`
- Parent staging lineage: `8413f83d3de57ec4e6530ce4781c3e5c644864d3`
- One-shot repair run: `34605772986`, job `103283703562`
- One-shot validation: rustfmt, `cargo test --locked --workspace --all-targets --no-fail-fast`, Clippy with `-D warnings`, rustdoc with `-D warnings`, and `git diff --check` all passed before publication. The temporary source-fix workflow removed itself in the published descendant.

The ordinary hosted suite compiles this ignored real-runtime acceptance but cannot establish its positive confinement result. The dedicated `[self-hosted, linux, cwl-hostile-workload, selinux]` lane remains the authority for executing the witness on an effective positive-LSM rootless Podman host.

## Acceptance contract

`tests/podman_rootless_e2e.rs::rootless_podman_effective_isolation_and_cleanup` now checks `/tmp` from inside the live container rather than trusting only Podman configuration metadata:

1. `/tmp` remains writable, proving the mount is usable for ephemeral workload state.
2. `/proc/self/mountinfo` must identify the effective `/tmp` mount as `tmpfs`.
3. `statvfs('/tmp')` must report capacity equal to the request-bound `tmpfs_bytes` limit.
4. Effective mount/superblock options must include `rw`, `noexec`, `nosuid`, and `nodev`.
5. A file created and marked executable on `/tmp` must still fail direct execution, providing a behavioral `noexec` witness rather than relying on option text alone.
6. The same acceptance continues to verify real cgroup-v2 `memory.max`, `pids.max`, and `cpu.max`, capability dropping, read-only root filesystem, egress denial, secret non-propagation, and leak-free container/network termination.

The positive-LSM lane must execute these checks on the same exact candidate used for the release decision. A predecessor result, static inspect output, or a fake-Podman fixture cannot close issue #35.

## Exact hosted evidence and baseline reconciliation

Exact `47ceddd966ea9d20b0784ac52dc7452d346eea44`, CI `34606000110`, made verify `103284586110` and hosted negative rootless/AppArmor `103284585789` GREEN. Coverage `103284586438` and branch coverage `103284586128` generated immutable artifacts and failed only the repository-wide 100% admissions: 4745/4867 lines (97.49%), 439/450 functions (97.56%), 6358/6574 regions (96.71%), and 677/716 branches (94.55%). Coverage digest is `sha256:3cd5226b715383b2051800fda4c900d16fddb88e4ac59e7653aeec36d1af0516`; branch digest is `sha256:2ac1dd6eae68ae42d85266ebd350a008767e09fead820208358d57266f1e8d22`. Dedicated positive SELinux job `103284586010` remained runner-unassigned, so this is compilation/hosted-negative/coverage evidence, not positive live `/tmp` acceptance.

One-shot baseline reconciliation run `34606332818`, job `103285543572`, passed rustfmt, the full workspace/all-target/no-fail-fast suite, Clippy, rustdoc, and `git diff --check`, then published ordinary descendant `9703c34a1600f804d809ee36eb926824aa429465` and removed its temporary workflow. `docs/product-technical-gap-baseline.md` now carries the exact `47ced...` evidence, the live `/tmp` witness contract, and the remaining positive-LSM/#35/#43/release gates without deleting the earlier causal ledger.

## Rationale

Podman's current `podman create` documentation states that a tmpfs mount without explicit alternatives uses `rw,noexec,nosuid,nodev`, and documents the tmpfs size as a byte limit. Linux kernel tmpfs documentation defines `size` as the allocation limit for the tmpfs instance. The acceptance therefore observes both the effective filesystem/mount options and the live capacity exposed by the kernel, then adds a direct execution counterexample for `noexec`.

## Release state

This change does **not** close #35. The checked-in witness is stronger, but positive execution is still absent while the dedicated SELinux runner is unavailable. #43's real over-lease termination evidence, repository-wide 100% owned-production coverage, qualifying independent review/security, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates.

## References

Podman Authors. (2026). *podman-create — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Linux Kernel Documentation. (n.d.). *Tmpfs*. Retrieved September 11, 2026, from https://docs.kernel.org/filesystems/tmpfs.html
