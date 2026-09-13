# Command Create Identity Traceability

Status: current owner-path evidence; release acceptance remains open.

## Decision boundary

The command runtime invokes Podman `create` with a runtime-owned `--cidfile`. The runtime must not let an independent stdout value silently redirect later lifecycle or destructive operations when a cidfile receipt exists. Podman documents both channels separately: `podman create` prints the container ID to stdout, while `--cidfile=file` writes the container ID to the specified file. The runtime therefore treats a present, well-formed cidfile receipt as lifecycle/destructive identity authority and requires successful-create stdout to agree with it.

This is narrower than full receipt-mandatory admission. The current implementation still accepts successful create when the cidfile is absent and falls back to the parsed stdout identifier for compatibility with legacy/debug fake backends. That fallback is a release hardening gap and means the repository has not yet fully enforced the TRD statement that the runtime-owned cidfile is the successful-create destructive lifecycle authority.

## Executed causal RED

Exact `d752b7343e4f5d298db40dd655fc9e2a032d7dca` added `tests/podman_command_execution_create_receipt_mismatch_red.rs`. Its fake Podman writes one valid 64-hex ID to the requested cidfile and prints a different valid 64-hex ID to stdout after a successful create.

Native CI `34632422106`, verify job `103372209100`, reached the intended security contradiction. Production used stdout as the acquired identity and continued to `podman init`, so the observed result was `BackendCommandFailed { operation: "container_init" }` instead of the expected `MalformedIsolationInspection { operation: "container_create_receipt" }`. The regression also requires that contradictory stdout never become cleanup authority and that cleanup target the runtime-owned receipt ID.

## Minimum causal repair

One-shot repair run `34633165627`, job `103374660299`, validated and published ordinary descendant `bbca444a874ea9bd1137e726c61c9043c53bfe66`. On successful create the adapter now:

1. parses stdout as independent create evidence;
2. reads the runtime-owned cidfile receipt;
3. uses the receipt when it is present and agrees with stdout;
4. fails closed with `container_create_receipt` when the two valid identities disagree;
5. cleans up only the receipt identity on that contradiction;
6. retains the explicit temporary compatibility fallback to stdout only when no receipt file exists.

The temporary source-fix workflow removed itself in the published descendant. The one-shot validation passed rustfmt, the focused create-identity regression, the output-encoding regression, full locked workspace/all-target tests, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check` before publishing.

## Fixture repair finding

The preceding one-shot attempt correctly exposed a test-fixture defect rather than a reason to weaken the production check. `tests/podman_command_execution_output_encoding_red.rs` wrote a non-Podman-like textual identifier (`fake-command-container-id`) into `--cidfile`. `read_command_create_receipt` deliberately admits only a 64-character lowercase hexadecimal container ID, so the stronger successful-create check failed before the intended invalid-UTF-8 log assertion.

Commit `366d42604421678614de08d81c6cde27a5ff8171` repaired that fake backend to use one consistent 64-hex identity in cidfile, stdout, and inspection evidence. Revalidation then preserved the original byte-faithful output tests while allowing the create-identity invariant to remain fail closed.

## Remaining acceptance

Do not mark this contract complete until successful create with a missing cidfile fails closed in production and all test/debug backends have migrated to the runtime-owned receipt contract. That future change needs its own RED before removing the compatibility fallback. Native exact-head CI also must execute a user-authored descendant because the bot-authored `bbca444...` PR run `34633277982` was `action_required` with zero jobs; the one-shot GREEN is causal pre-publish evidence, not transferable native exact-head GREEN.

This work does not discharge the independent release gates for repository-wide 100% owned-production coverage, positive effective-LSM, issues #35/#43 real-runtime acceptance, independent review/security, protected-head verification, or immutable release/SBOM/provenance/reproducibility/rollback evidence.

## References

Podman. (2026). *podman-create — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Repository authority: `docs/TRD.md`; PR #112; exact RED `d752b734...`; repair `bbca444...`.
