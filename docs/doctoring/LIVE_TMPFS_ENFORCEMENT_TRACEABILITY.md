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

## Rationale

Podman's current `podman create` documentation states that a tmpfs mount without explicit alternatives uses `rw,noexec,nosuid,nodev`, and documents the tmpfs size as a byte limit. Linux kernel tmpfs documentation defines `size` as the allocation limit for the tmpfs instance. The acceptance therefore observes both the effective filesystem/mount options and the live capacity exposed by the kernel, then adds a direct execution counterexample for `noexec`.

## Release state

This change does **not** close #35. The checked-in witness is stronger, but positive execution is still absent while the dedicated SELinux runner is unavailable. #43's real over-lease termination evidence, repository-wide 100% owned-production coverage, qualifying independent review/security, protected-head verification, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates.

## References

Podman Authors. (2026). *podman-create — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Linux Kernel Documentation. (n.d.). *Tmpfs*. Retrieved September 11, 2026, from https://docs.kernel.org/filesystems/tmpfs.html
