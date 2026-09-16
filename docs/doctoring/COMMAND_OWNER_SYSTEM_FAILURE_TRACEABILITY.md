# Command-owner system failure traceability

Last reviewed: 2026-09-13

## Causal coverage finding

Exact native branch artifact `10302873384` on `a28803f47aad499a6c411596831ca365971076df` measured 5068/5129 lines, 484/486 functions, 6805/6956 regions, and 712/720 branches. Source-coordinate grouping of `src/infrastructure/podman.rs` isolated its two wholly unexecuted function outcomes: the error mapper for command create-receipt temporary-directory creation at the former line 613, and the OS-entropy error mapper for `command_sandbox_identity` at the former line 1809. Both are command/runtime owner responsibilities, not application-service lifecycle/readiness behavior.

The first source-fix staging run for these two findings, `34710805259` on `7002008f155ec8ddd5583e0ea06b919d3fafa86e`, failed before modifying source because its authoring script matched a multiline Rust fragment with indentation-sensitive literal text. Validation and publication were skipped, and the failed workflow was removed at `efedacef91d56793ebd2ba4bfac1b2e04a708d45` before the replacement was staged.

## Decision

Command create-receipt setup continues to use `tempfile::Builder::tempdir()` in production. The returned `std::io::Result<TempDir>` is passed through a provider-neutral resolver whose failure remains `BackendInvocationFailed { operation: "container_create_receipt" }`. The test supplies an explicit `io::Error` value instead of mutating `TMPDIR`, permissions, or process-global filesystem state.

Execution identity continues to obtain a 16-byte nonce from `getrandom::fill`. Only the already-existing success/failure decision is extracted into `require_execution_identity_entropy`; a failed entropy call returns `BackendInvocationFailed { operation: "execution_identity" }` before the nonce can be hashed into an identity. No deterministic nonce, retry, fallback entropy, panic, unsafe path, or coverage exclusion is introduced.

## Candidate validation and authority boundary

Replacement one-shot run `34710910038`, job `103599421882`, executed exact staging head `5dc2450d7439526fa7d931c394e8f5b0416a4758`. Exact-head ownership verification, structural source authoring, repository validation, rustfmt, both focused system-failure regressions, the full locked workspace/all-target suite, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check` all completed successfully. The workflow removed itself and published bot-authored source candidate `fb71ec4ff78ecc3c5ebe689d08a68ae05539cf31`.

That one-shot result establishes causal candidate validation only. It is not transferable merge or release authority. This connector-authored documentation descendant exists to reacquire ordinary native exact-head CI and immutable coverage over the same production source tree plus this authority record. The prior 484/486 repository function deficit is not considered closed until that descendant's native coverage proves the result.

## Rejected alternatives

Rejected approaches are process-global temporary-directory mutation, permission races, host RNG fault injection, retry loops, deterministic fallback randomness, weakening exact error taxonomy, synthetic application-service fixtures, and denominator/exclusion changes. Those approaches either create scheduler-dependent tests or cross the canonical owner boundary.

## Acceptance

The exact ordinary descendant must pass repository validation, rustfmt, the full locked workspace/all-target suite, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check`. Native exact-head coverage must then prove the prior 484/486 function deficit is closed or identify the remaining functions without transferring application-service-owned gaps into this lane. Positive effective-LSM, qualifying independent review/security, #21/#113 integration, #35/#43 real-runtime evidence, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates.

## References

Allen, S., The Rust Project Developers, Mannix, A., & White, J. (2026). *tempfile 3.27.0: Builder::tempdir*. Docs.rs. https://docs.rs/tempfile/latest/tempfile/struct.Builder.html

The Rand Project Developers. (2025). *getrandom 0.3.4: System's random number generator*. Docs.rs. https://docs.rs/crate/getrandom/0.3.4
