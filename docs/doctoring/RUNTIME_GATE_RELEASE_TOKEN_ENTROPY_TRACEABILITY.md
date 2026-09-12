# Runtime-gate release-token entropy failure traceability

Last reviewed: 2026-09-13

## Problem and causal RED

Exact command-owner source head `c31b569f9d65ba0960b0358fca2c8dc2e8c0e788` retained one uncovered production function outcome in `src/infrastructure/podman_runtime_gate_binding.rs`: failure of `getrandom::fill` while creating the one-time release token. Native CI `34709222684` made verify `103594820336` and hosted negative rootless/AppArmor `103594820266` GREEN. Coverage artifacts were generated and failed only repository-wide 100% admissions. Exact totals were 5042/5109 lines, 479/482 functions, 6772/6927 regions, and 710/718 branches; runtime-gate binding was 454/462 lines, 55/56 functions, 652/666 regions, and 22/22 branches.

This outcome is a real fail-closed security path. The pinned `getrandom 0.3.4` contract returns an error instead of known-insecure bytes when the system entropy source fails. A release token therefore must not fall back to deterministic or guessed bytes.

The first source-fix workflow at `dd917bd93c508d9aa26d0c7b0f5783bef83f5a01` did not alter source: run `34709675809`, job `103596002400`, failed in its authoring step because the encoded test payload contained an invalid literal `\r` between `super::` and `runtime_gate_release_token_with`. Validation and publication were skipped. That workflow was removed before this repair was restaged.

## Decision

Production continues to call the operating-system entropy source exactly once. `fill_runtime_gate_nonce` exposes only success/failure from `getrandom::fill`; `runtime_gate_release_token_with` is a private deterministic seam for testing the decision. On failure it returns the existing provider-neutral `BackendInvocationFailed { operation: "runtime_gate_release_token" }`. On success it hashes the complete 32-byte nonce exactly as before.

Rejected alternatives are retry loops, deterministic fallback entropy, environment mutation, host RNG fault injection, panic/unsafe paths, and coverage exclusion.

## Acceptance

The exact descendant must pass repository validation, rustfmt, focused entropy tests, the full locked workspace/all-target test suite, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check`. Native exact-head coverage must then show the former 55/56 runtime-gate function deficit closed without reducing the 22/22 branch result. Positive effective-LSM, independent review/security, #35/#43 real-runtime acceptance, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates.

## Primary reference

The Rand Project Developers. (2025). *getrandom 0.3.4: System's random number generator*. Docs.rs. https://docs.rs/crate/getrandom/0.3.4
