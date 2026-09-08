# LLVM Source-Region Coverage Traceability

## Decision status

Proposed on 2026-09-08. Issue #93 owns the coverage-admission repair above root issue #83. This document records stable source/tool evidence; transient queue and runner identifiers remain PR/Issue metadata.

## Problem and exact evidence

Exact root predecessor `8e51c28eea6f31a19f13811e926574f5d44ae757` produced native branch-coverage artifact `10034315670`. LLVM's exported production summary reports `regions=2636/2637`, with `src/infrastructure/bounded_command.rs=440/441`, while functions, lines, and the then-current branch totals are independently reported.

The same JSON is internally more specific than that aggregate summary. Auditing every function region whose file ID resolves to one of the eight production file records yields exactly 2,637 unique code-region source coordinates. Every one of those 2,637 coordinates has at least one positive execution count. For `bounded_command.rs`, the function-region surface contains exactly 441 unique source coordinates and every coordinate has an executed instance. The file-level `segments` surface also contains no zero-count counted region entry for the alleged missing region.

The discrepancy is concentrated in generic/monomorphized infrastructure helpers. Forty-three `bounded_command.rs` source coordinates have both zero-count and positive-count function-region instances across compiled instantiations, including `captured_pipes`, `supervise_child`, `drain_stream`, `join_stream`, `finalize_output`, and `kill_and_reap`. A source coordinate can therefore be executed while one generated instantiation of that coordinate remains unexecuted.

## Authority and interpretation

LLVM source-based coverage distinguishes source coverage from instantiation coverage. Its documentation states that function coverage treats a function as executed when any instantiation executes, while instantiation coverage is a separate statistic for the individual generated instantiations. It defines region coverage as the percentage of code regions executed at least once. `llvm-cov export` publishes regions, functions, branches, expansions, and summary information in JSON.

For this repository's requirement of 100% owned production **source-region** coverage, the admission gate therefore must answer whether every production source region executes at least once. It must not silently turn that requirement into 100% execution of every compiler-generated generic instantiation. If the product later requires instantiation coverage as an independent quality metric, that requires an explicit requirement and gate rather than overloading source-region coverage.

## Selected measurement contract

For the region gate only:

1. Read the complete function-region surface from the same `llvm-cov export` JSON used by the existing gate.
2. Restrict attribution to filenames present in the production `files` surface.
3. Identify a source code region by production filename, start line/column, end line/column, and LLVM region kind.
4. Collapse duplicate instances of the same source coordinate. The source region is covered when any attributed instance has an execution count greater than zero.
5. Cross-check the derived unique source-region denominator against the sum of production file-summary region counts. A mismatch is malformed/inconsistent evidence and fails closed.
6. Keep LLVM's raw aggregate summary in diagnostics. Do not rewrite or conceal it.
7. Keep line, function, branch, zero-branch, source-file attribution, and zero-count segment diagnostics unchanged.

This does not lower the threshold: the required source-region result remains exactly 100%. It prevents a compiler-instantiation accounting detail from creating an unrepairable false negative when the export's own source-coordinate evidence proves complete execution.

## RED and minimum GREEN

Test-only commit `084b592c841c1240596120a9c8cad80fd5878d42` requires three behaviors before production checker changes:

- one shared source region with zero-count and positive-count instantiations is `1/1` covered;
- a source region whose every instance is zero is `0/1` covered;
- a derived denominator that disagrees with production file summaries raises an evidence error.

The current checker has no `_source_region_counts` contract, so this head is intentionally RED. After causal execution, the minimum GREEN is limited to deriving/cross-checking source-region counts and using them for the `regions` admission metric. No threshold reduction, file exclusion, impossible fixture, runtime predicate removal, or unrelated coverage change is authorized.

## Rejected alternatives

1. Lower region coverage below 100%. Rejected: contradicts repository quality authority and can hide real gaps.
2. Exclude `bounded_command.rs` or generic helpers. Rejected: these are owned production security-runtime code.
3. Add tests only to execute every monomorphization. Rejected as the repair for this finding: the product requirement is source-region coverage, LLVM exposes instantiation coverage separately, and exact artifact evidence already shows every source coordinate executed.
4. Trust aggregate `totals.regions` without checking detailed export evidence. Rejected: exact artifact `10034315670` demonstrates an aggregate/detail inconsistency that cannot identify a missing source region.
5. Ignore the raw LLVM summary. Rejected: raw tool output remains traceable evidence and must be reported alongside the canonical source-region derivation.

## References

LLVM Project. (2026). *llvm-cov—emit coverage information*. https://llvm.org/docs/CommandGuide/llvm-cov.html

LLVM Project. (2026). *Source-based code coverage*. Clang documentation. https://clang.llvm.org/docs/SourceBasedCodeCoverage.html
