# Coverage source-coordinate admission traceability

Last reviewed: 2026-09-15 KST

## Problem

The repository uses LLVM source-based coverage as a release admission, but Rust monomorphization/codegen can export more than one function-region instance for the same physical source coordinate. Treating every exported instance as an independent source line or source region can therefore report a zero-count instance as uncovered even when another instance of the same source coordinate executed. That is a measurement-identity problem, not evidence that the underlying source decision was untested.

The admission must still fail closed. It must not drop a source coordinate, shrink the denominator by configuration, ignore a zero-count source coordinate, or replace branch coverage with line coverage.

## Canonical decision

`scripts/check_coverage.py` uses physical source identity for line and region admission while preserving LLVM's raw totals in the diagnostic output.

For source regions, identity is the tuple `(filename, line_start, column_start, line_end, column_end, region_kind)`. Multiple exported instances of that same tuple are unioned only for execution state: the source region is covered when at least one instance executed. The derived source-region denominator must equal the sum of the production file region summaries; disagreement is an error.

For source lines, function-region attribution is first unioned by `(filename, line_number)`. The result is independently reconstructed from LLVM file segments. The two representations must have identical line identities and identical executed/unexecuted truth; any disagreement fails admission. Gap-region counts are used only when no non-gap region exists on the line, matching LLVM's documented line-count semantics.

Functions and branches retain their existing LLVM admission. In particular, this change does not reinterpret or reduce branch conditions.

## Causal execution

The source-coordinate region admission was already present in canonical application-service owner PR #21. PR #112 adopted that released-in-lineage contract rather than maintaining a second interpretation.

A later coverage investigation found the same duplicate-instance identity problem affecting the raw line metric. Purpose-limited workflow `Command source-line coverage repair` executed successfully on exact predecessor `a27b510d50ebe424717fc74b9cb98109f024c3c5` (run `34751853314`). It validated the source-line union, cross-checked function regions against file segments, ran repository coverage-contract tests, and published `19f12cea9f39b4d22214b01ba9ca6000db46c94c` with the temporary authoring helper and workflow removed.

The publication commit was authored by `github-actions[bot]`, so its pull-request CI materialized as `action_required` rather than usable exact-head product evidence. This traceability commit is an ordinary owner-authored descendant whose purpose is to preserve the decision record and reacquire native CI on a human-authored exact head. No predecessor GREEN transfers.

## 2026-09-15 review repair: terminal line at column 1

Fresh review identified an inconsistency between the two source-line reconstructions. The file-segment path already treated a multi-line counted interval ending at column 1 as not touching the terminal line, while `_source_line_map_from_functions` unconditionally iterated through `line_end`. A normal region from `10:5` to `11:1` therefore produced function lines `{10, 11}` but segment lines `{10}`, causing the fail-closed denominator check to reject valid LLVM evidence.

Checked-in regression `f100d97b6d99a773014ee679b9300bbae89c65be` fixes the measurement expectation before the implementation repair: the `10:5 → 11:1` region must account for exactly one physical source line. A deterministic local reconstruction reproduced the old disagreement. Commit `af2ea04ff29332c81fd78eecde4f030e20992169` then applies the same interval rule to function regions: include all lines before `line_end`; include `line_end` only for a same-line region or when `column_end > 1`. This preserves ordinary same-line ranges and multi-line ranges that contain source text on their final line.

The rule is derived from the source-coordinate boundaries exported by LLVM and cross-validated against LLVM's own file-segment representation; it is not a coverage exclusion or denominator reduction. LLVM's current coverage-mapping documentation defines mapping regions by start/end line and column source ranges and states that code regions drive line execution counts. `llvm-cov export` exposes the region/function/file data consumed by this admission. The repository therefore keeps the independent function-region/segment agreement check as the stronger local invariant.

## Acceptance invariants

- Raw LLVM line/region totals remain visible in logs.
- Source-region identity includes file, exact start/end coordinates, and region kind.
- Region denominator mismatch against production file summaries fails closed.
- Source-line identity and execution truth must agree between function regions and file segments.
- A multi-line source interval ending at column 1 does not claim the terminal line solely from its end coordinate.
- Mixed zero/nonzero codegen instances for one source coordinate count as one executed physical source coordinate.
- All-zero instances for one source coordinate remain uncovered.
- Branch and function admissions are not weakened or redefined.
- No `#[coverage(off)]`, ignore regex, sample reduction, warm-cache substitution, or denominator configuration is introduced to obtain 100%.

## Evidence and authority

LLVM describes coverage mapping as a mapping from profiling counters to source ranges and states that mapping is per function. A mapping region carries a source range, file ID, counter, and region kind; code regions are used to compute line execution counts and coverage statistics. LLVM also documents that source regions may have multiple instantiations and that `llvm-cov show` can present each instantiation separately alongside a combined summary. These properties justify source-coordinate identity plus execution union while requiring independent denominator checks.

### References

LLVM Project. (n.d.). *LLVM code coverage mapping format*. Retrieved September 15, 2026, from https://llvm.org/docs/CoverageMappingFormat.html

LLVM Project. (n.d.). *llvm-cov — emit coverage information*. Retrieved September 15, 2026, from https://llvm.org/docs/CommandGuide/llvm-cov.html

## Remaining release boundary

This measurement repair does not establish release readiness by itself. PR #112 still requires exact-head native CI after the latest owner-authored descendant, dedicated positive effective-LSM evidence, stable #21/#113 application-service ancestry for its owned readiness/lifecycle findings, qualifying independent review and security gates, real-runtime acceptance for #35/#43, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback evidence.
