# Coverage source-branch union traceability

Last reviewed: 2026-09-18

## Finding

PR #125 exact `f7484afcdf41d2050bee199bf96bfd9243fb4b1c` produced real nightly branch-coverage evidence in workflow run `35094825945`. The branch-coverage artifact `10463832719` (`sha256:7e3886501fb1b1f4bd75a9aba7887468cdaaab391bd761bea87e2e7b06a70d38`) reports `src/infrastructure/bounded_command.rs` at 25/26 branch outcomes even though its exported per-function records contain 13 physical branch source ranges and, after grouping repeated codegen records by exact source range, both the true and false outcome counters are non-zero for all 13 ranges. The physical denominator remains 26; the mismatch is in the covered numerator across repeated Rust codegen instantiations.

The same artifact reports raw line and region deficits in `bounded_command.rs`, but current repository-fitness authority already established that raw LLVM codegen-instance summaries can understate physical source coverage. Source-line and source-region admission therefore union execution at the physical source identity while preserving the production-file denominator and failing closed when the derived denominator disagrees. Branch admission still consumes LLVM's raw summary unchanged, so it can contradict the repository's own source-identity rule.

This is not evidence that the 100% branch requirement should be weakened. The target invariant is still: every true and false outcome of every physical production branch must execute at least once. Duplicate monomorphized/codegen records must not multiply or split that physical obligation.

## RED

Test-only commit `2f621cb4f6c561550cc0caafcc44a846b78a3fa5` adds `tests/coverage_branch_codegen_union_red.rs`. The witness gives two exported function/codegen records the same physical branch source range. One record executes only the true outcome and the other executes only the false outcome while the raw summary remains 1/2. A source-identity admission contract must report the physical branch as 2/2 and pass. A negative control leaves the false outcome unexecuted in every instance and must still fail at 1/2.

The RED intentionally drives the existing `scripts/check_coverage.py --require-branches` entry point instead of importing a not-yet-implemented helper. That keeps the failure semantic: current policy trusts the raw branch summary and rejects the complementary-instantiation case. No production Rust, threshold, ignore rule, or denominator changes in this commit.

## Minimum causal repair after executed RED

After the exact RED executes behind repository/fmt prerequisites, the minimum repair is to derive branch coverage from exported per-function `branches` records using physical source identity `(filename, line_start, column_start, line_end, column_end, region_kind)` and union the two outcome counters across repeated codegen instantiations. The derived physical-outcome denominator must equal the sum of production-file branch denominators; any mismatch must fail closed. Diagnostics must likewise report source-branch outcomes rather than reintroduce raw per-instantiation false deficits.

Do not remove branch instrumentation, lower the 100% threshold, ignore `bounded_command.rs`, or manufacture warm-up execution merely to satisfy the metric. #125 remains the bounded-command behavior owner; #112 owns this repository-fitness interpretation because the defect is in coverage admission, not in bounded-command runtime semantics.

## Primary references

LLVM Project. (n.d.). *LLVM code coverage mapping format*. Retrieved September 18, 2026, from https://llvm.org/docs/CoverageMappingFormat.html

LLVM Project. (n.d.). *llvm-cov—emit coverage information*. Retrieved September 18, 2026, from https://llvm.org/docs/CommandGuide/llvm-cov.html

LLVM's coverage mapping format defines a branch region by a source range and two counters tracking true and false evaluation. `llvm-cov` also documents separately displayed source instantiations and a combined summary for repeatedly instantiated source regions. Those two facts support keeping the physical branch source range as the admission identity while unioning execution evidence across codegen instances.
