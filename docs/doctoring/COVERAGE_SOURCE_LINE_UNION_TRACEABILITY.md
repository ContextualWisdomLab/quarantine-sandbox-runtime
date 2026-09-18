# Source-line Coverage Union Traceability

Last reviewed: 2026-09-13

## Problem and causal evidence

Exact predecessor `d82fa32a5fd79e30a2b0fb6e986e4701c5c137e8` produced branch artifact `10314340212` (`sha256:76487350507dcddd4a8e84e323cf37bdec9c22dd073d969dbe9eeff09a84189a`). LLVM reported 5260/5298 raw lines. Source-coordinate inspection of that immutable artifact showed that `src/main.rs` had 465/476 raw lines while source-union diagnostics isolated only two genuinely uncovered physical lines: the alternate-architecture and unsupported-architecture arms of the test-only ELF fixture. `src/main.rs` remained unchanged through causal predecessor `b760ccf4f01dc2735e0326b0a343e03c74a06a8a`.

The same artifact exposes the upstream Rust/LLVM multi-instantiation accounting problem already handled for source regions: one physical source location may execute in one monomorphization while another zero-count instantiation remains in the raw summary. LLVM defines line coverage by whether a code line executed at least once. Rust issue rust-lang/rust#137524 documents a reproducible multi-instantiation case where raw totals retain an uncovered line although another instantiation executes the same source and exported JSON cannot identify a corresponding zero-count physical region.

## Decision

Admission preserves and prints LLVM raw line totals, then separately unions physical source-line execution across function instantiations. It independently reconstructs the same source-line map from file-level coverage segments and fails closed if denominator or execution truth disagrees. No line, file, or metric is excluded and no denominator is sampled or manually reduced.

The test-only runtime-gate ELF fixture no longer models mutually exclusive host architectures as a runtime `match`. Architecture selection is compile-time `cfg`, so each supported target contains only the machine value that can execute. Unsupported targets still fail explicitly if this test helper is compiled. Production runtime behavior, public contracts, and supported x86_64/aarch64 machine values are unchanged.

## Validation contract

The repair must pass source-line regression tests for mixed and all-zero instantiations plus mapping disagreement, rustfmt, the full locked workspace/all-target test suite, Clippy with warnings denied, rustdoc with warnings denied, and branch coverage generation. Exact generated evidence must show no genuinely uncovered physical source line in `src/main.rs`; remaining source-line and branch deficits must stay attributable to real application-service owner paths rather than codegen-instance artifacts.

## References

LLVM Project. (2026). *Source-based code coverage*. Clang documentation. https://clang.llvm.org/docs/SourceBasedCodeCoverage.html

LLVM Project. (2026). *llvm-cov - emit coverage information*. LLVM documentation. https://llvm.org/docs/CommandGuide/llvm-cov.html

Rust Project. (2025). *Test Coverage: cannot find uncovered lines and regions* (rust-lang/rust#137524). GitHub. https://github.com/rust-lang/rust/issues/137524
