# LLVM Source-Region Coverage Traceability

## Decision status

Proposed on 2026-09-08. Issue #93 and Draft PR #94 own this coverage-admission repair above root issue #83. The repair has causal RED evidence and hosted candidate GREEN on dependency-safe ancestry, but it is not protected-head or release authority until normal integration and the remaining independent release gates complete.

## Problem and exact evidence

Exact root predecessor `8e51c28eea6f31a19f13811e926574f5d44ae757` produced branch-coverage artifact `10034315670`. LLVM's exported production summary reported `regions=2636/2637`, with `src/infrastructure/bounded_command.rs=440/441`, while functions, lines, and branches were independently `189/189`, `1974/1974`, and `456/460`.

Auditing every function region whose file ID resolves to one of the eight production file records yielded exactly 2,637 unique code-region source coordinates and every coordinate had at least one positive execution count. `bounded_command.rs` contained exactly 441 unique source coordinates and all 441 were executed. Forty-three of those coordinates had both zero-count and positive-count instances across generic/monomorphized helpers such as `captured_pipes`, `supervise_child`, `drain_stream`, `join_stream`, `finalize_output`, and `kill_and_reap`. The file-level segment surface contained no zero-count counted region entry for the alleged missing source region.

The later hosted exact candidate `4990976015c554df1cc485135dfa1d5e3a38e4ca` reproduced the same class after the root's parser-dominated Podman branch simplification: LLVM raw report showed `bounded_command.rs=440/441`, total raw regions `2629/2630`, functions `189/189`, lines `1973/1973`, and branches `452/452`, while the canonical checker reported source regions `2630/2630`. The branch gate stayed independently exact at `452/452`.

## Authority and interpretation

LLVM source-based coverage distinguishes source coverage from instantiation coverage. Its documentation treats individual instantiations as a separate statistic and defines region coverage around code regions executed at least once. `llvm-cov export` publishes detailed regions, functions, branches, expansions, and summaries.

LLVM's JSON exporter represents each region as `[LineStart, ColumnStart, LineEnd, ColumnEnd, ExecutionCount, FileID, ExpandedFileID, Kind]`. The repository can therefore reconcile source-region truth from the same immutable evidence object using production filename, source start/end coordinate, and region kind, while retaining raw LLVM summaries for traceability.

For this repository's requirement of 100% owned production **source-region** coverage, the admission gate answers whether every production source region executes at least once. It does not silently redefine that requirement as 100% execution of every compiler-generated generic instantiation. If instantiation coverage becomes a product requirement, it must be specified and gated separately.

## Selected measurement contract

For the region gate only:

1. Read the complete function-region surface from the same `llvm-cov export` JSON used by the existing gate.
2. Restrict attribution to filenames present in the production `files` surface.
3. Identify a source code region by production filename, start line/column, end line/column, and LLVM region kind.
4. Collapse duplicate compiler instances of the same source coordinate. The source region is covered when any attributed instance has an execution count greater than zero.
5. Cross-check the derived unique source-region denominator against the sum of production file-summary region counts. A mismatch is malformed/inconsistent evidence and fails closed.
6. Keep LLVM's raw aggregate summary in diagnostics; do not rewrite or conceal it.
7. Keep line, function, branch, zero-branch, source-file attribution, and zero-count segment diagnostics independently fail closed.

The threshold remains exactly 100%.

## RED lineage, review, and minimum repair

Initial test-only commit `084b592c841c1240596120a9c8cad80fd5878d42` required three helper-level behaviors: mixed zero/nonzero instantiations of one source coordinate are `1/1` covered, an all-zero source coordinate remains `0/1`, and denominator disagreement raises an evidence error.

Exact review `5136297577` found that this RED did not bind the actual release path: `_source_region_counts` could be implemented while `main()` continued to gate raw `totals.regions`. Test-only commit `31b7e7b40a374122c938089e374bd6303e7e9c4a` therefore added admission-level REDs requiring `main()` to retain `regions (LLVM raw): 0/1`, admit canonical `regions: 1/1` when the shared source coordinate executes in any instance, and fail closed on denominator disagreement.

Materialized exact-content local execution reproduced both causal layers. The first test-only head failed because `_source_region_counts` did not exist. A helper-only trial made helper tests pass but left both `main()` admission tests RED, proving the release-path defect independently.

Minimum checker commit `1454644358190356aa5a84080129d24a1bcac77e` adds `_source_region_counts` and changes only the `regions` admission metric. Raw LLVM regions remain visible; canonical source regions drive only the region verdict; denominator disagreement fails closed; line/function/branch/zero-branch and file/segment diagnostics remain intact.

## Hosted exact evidence and root repair

At exact #94 head `4990976015c554df1cc485135dfa1d5e3a38e4ca`, native CI `34175975169` executed the candidate. Coverage `101905358629`, branch coverage `101905358853`, and hosted negative rootless/AppArmor `101905358765` completed successfully. The coverage parser's eight tests were GREEN in verify. Branch coverage generated artifact `10037275727` (`sha256:41be203dce7d8588f2ab450cf63983a9f4053515afad35762e86bfcb466bc189`) and the checker reported `lines=1973/1973`, `functions=189/189`, raw LLVM `regions=2629/2630`, canonical source regions `2630/2630`, and branches `452/452`. The raw `bounded_command.rs=440/441` diagnostic stayed visible.

Verify `101905358746` failed only at `cargo fmt --check`: the inherited root file `tests/root_coverage_reachable_edges.rs` required rustfmt wrapping of one long `process_top` assignment. Repository policy and all eight checker tests had already passed before that failure. The failure was therefore repaired at the canonical root, not hidden in the coverage child. Root commit `489af256d8e7cdac777f8ece5f50829f4238b35f` applies only that rustfmt delta.

Draft #94 then adopted the root repair by ordinary two-parent, non-force merge commit `b981dbc4e28b41690fde4d233321f1dfe9eaf377`, preserving both the coverage child and root lineage. On that exact integrated child head, native CI `34176200159` has verify `101905976342`, coverage `101905976423`, branch coverage `101905976383`, and hosted negative rootless/AppArmor `101905976218` all GREEN. The dedicated positive-LSM job `101905976348` remains queued on `[self-hosted, linux, cwl-hostile-workload, selinux]`; it is not replaced by hosted negative evidence.

## Rejected alternatives

1. Lower region coverage below 100%. Rejected because it contradicts repository quality authority and can hide real gaps.
2. Exclude `bounded_command.rs` or generic helpers. Rejected because they are owned production security-runtime code.
3. Add tests only to execute every monomorphization. Rejected as the repair for this finding because the product requirement is source-region coverage and exact detailed evidence already proves every source coordinate executes.
4. Trust aggregate `totals.regions` without detailed reconciliation. Rejected because two exact artifacts demonstrate aggregate/detail disagreement without an attributable missing source region.
5. Ignore the raw LLVM summary. Rejected; raw tool output remains traceable evidence beside the canonical source-region derivation.
6. Implement only a helper without binding it to `main()`. Rejected by review `5136297577` because that would leave release admission semantically unchanged.
7. Repair the root-owned formatting failure only in the child. Rejected because that would duplicate/diverge root truth; the formatting fix landed on the canonical root and was non-force adopted.

## References

LLVM Project. (2026). *llvm-cov—emit coverage information*. https://llvm.org/docs/CommandGuide/llvm-cov.html

LLVM Project. (2026). *Source-based code coverage*. Clang documentation. https://clang.llvm.org/docs/SourceBasedCodeCoverage.html

LLVM Project. (2026). *CoverageExporterJson.cpp* [Source code]. llvm-project. https://github.com/llvm/llvm-project/blob/main/llvm/tools/llvm-cov/CoverageExporterJson.cpp
