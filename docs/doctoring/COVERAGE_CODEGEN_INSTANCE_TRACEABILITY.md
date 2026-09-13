# Coverage codegen-instance traceability

Last reviewed: 2026-09-13

PR #112 exact `e458677ed4cadd7a4270a92ea8148a11444034f3`, workflow run `34747796298`, job `103698811973`, tested whether replacing generic staging I/O dispatch with `&dyn RuntimeGateStagingIo` would close four reported regions. Formatting, full tests, Clippy, rustdoc, repository policy, and post-fix coverage generation passed, but the post-fix assertion failed because `runtime_gate_artifact.rs` remained `514/518` raw regions. Publication steps were skipped, so the candidate production-source change was never committed.

Artifact `10315155390` (`sha256:f687e39221bfca213f58a21b92f4839d4f49e2398972ddff0a20c818c3dccc78`) reports `5260/5298` lines, `509/509` functions, and `7033/7159` raw regions. The four zero-count staging records share exact source coordinates with regions executed by existing deterministic `ScriptedStagingIo` witnesses in another instrumented function/codegen record. Exact source-coordinate aggregation therefore changes the region numerator to `7076` while preserving the denominator at `7159`; `runtime_gate_artifact.rs` is `518/518` by source coordinate.

`scripts/check_coverage.py` now combines duplicate codegen records only for the region covered numerator. LLVM file-summary region totals remain denominator authority. Reconstructed `(file, start line, start column, end line, end column, region kind)` counts must equal LLVM file totals or admission fails closed. Lines, functions, and branches retain existing semantics. No exclusion, ignore attribute, denominator reduction, or production-runtime weakening is introduced. Tests cover executed duplicates, all-zero duplicates, file-ID attribution, and fail-closed denominator mismatch. The falsified self-modifying staging workflow is removed.

LLVM coverage mapping defines mapping regions by source range, file identity, counter, and region kind. `llvm-cov` separately exposes multiple source instantiations and their combined presentation, which is why source identity rather than one codegen-instance counter is the relevant admission unit here.

## References

LLVM Project. (n.d.). *LLVM code coverage mapping format*. Retrieved September 13, 2026, from https://llvm.org/docs/CoverageMappingFormat.html

LLVM Project. (n.d.). *llvm-cov—emit coverage information*. Retrieved September 13, 2026, from https://llvm.org/docs/CommandGuide/llvm-cov.html
