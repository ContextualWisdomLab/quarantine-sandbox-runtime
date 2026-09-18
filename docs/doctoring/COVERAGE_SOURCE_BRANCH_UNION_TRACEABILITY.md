# Coverage source-branch union traceability

Last reviewed: 2026-09-19

## Finding

PR #125 exact `f7484afcdf41d2050bee199bf96bfd9243fb4b1c` produced real branch-coverage evidence in workflow run `35094825945`. The branch-coverage artifact `10463832719` (`sha256:7e3886501fb1f4bd75a9aba7887468cdaaab391bd761bea87e2e7b06a70d38`) reports `src/infrastructure/bounded_command.rs` at 25/26 raw branch outcomes even though its exported per-function records contain 13 physical branch source ranges and, after grouping repeated codegen records by exact source range, both the true and false outcome counters are non-zero for all 13 ranges. The physical denominator remains 26; the defect is the covered numerator across repeated Rust codegen instantiations.

The repository already treats source lines and source regions as physical-source obligations rather than independent codegen-instance obligations. Branch admission had remained different: `scripts/check_coverage.py --require-branches` trusted LLVM's raw branch summary and per-file raw branch summary directly. That could reject fully exercised source behavior when complementary true/false execution was split across monomorphized instances.

The invariant remains strict: every true and false outcome of every physical production branch must execute at least once. Duplicate codegen records must neither multiply the denominator nor split one physical obligation into an artificial deficit.

## Executed RED

Test-only commit `2f621cb4f6c561550cc0caafcc44a846b78a3fa5` added `tests/coverage_branch_codegen_union_red.rs`. Docs-only exact `48ad86148be70fff165800d07221409bc7521af5` then ran native CI `35257533087`.

Verify job `105324769660` passed exact checkout, dependency lock, repository policy, CI-evidence contracts and rustfmt, then executed the full locked workspace/all-target suite. The dedicated witness failed for the intended reason: two exported function/codegen records described the same physical branch; one covered only the true outcome and the other covered only the false outcome, while the policy still emitted `branches: 1/2` and `branches coverage is 1/2`. Other tests continued under `--no-fail-fast`. Hosted negative rootless/AppArmor was GREEN. Positive SELinux remained without an eligible runner and was cancelled. Coverage and branch-coverage jobs also reached the test suite and failed while this intentional RED was active.

This is assertion-level causal evidence. It is not a request to lower the 100% branch threshold, ignore `bounded_command.rs`, exclude codegen instances, or manufacture warm-up execution.

## Minimum causal repair

Commit `89c182151051668f4f468c53e64a947d965dda95` changes the repository coverage policy, not Rust runtime behavior. `scripts/check_coverage.py` now derives branch admission from exported per-function `branches` records using physical identity `(filename, line_start, column_start, line_end, column_end, region_kind)`. For each identity it unions the two LLVM execution counters independently across codegen instances. The physical-outcome denominator remains two outcomes per physical branch.

The repair fails closed if the derived physical denominator for any production file differs from that file's exported branch-summary denominator. Per-file diagnostics also use the same physical branch outcome counts, so a raw per-instantiation deficit cannot reappear only in diagnostics after aggregate admission has been repaired. The regression witness retains its complementary-execution positive control and all-instance-uncovered negative control and adds a denominator-mismatch fail-closed control.

Native CI `35379390471` has materialized for exact `89c182151051668f4f468c53e64a947d965dda95` and is queued. Do not promote this repair to GREEN until an unchanged exact descendant executes repository/fmt/full tests/Clippy/rustdoc and the coverage admission path successfully. Positive effective-LSM and release gates remain independent.

## Ownership and release boundary

#112 owns this defect because it is repository-fitness interpretation, not bounded-command runtime semantics. #125 remains the bounded-command behavior owner. The branch-union repair must not be copied into unrelated leaves, and predecessor success cannot authorize a moved #112 head.

The coverage source-identity rule is admission logic only. It does not change instrumentation, source code execution, exclusion policy, the 100% requirement, or the physical branch denominator. Immutable release authority still requires the normal protected integration and release evidence path.

## Primary references

LLVM Project. (n.d.). *LLVM code coverage mapping format*. Retrieved September 19, 2026, from https://llvm.org/docs/CoverageMappingFormat.html

LLVM Project. (n.d.). *llvm-cov—emit coverage information*. Retrieved September 19, 2026, from https://llvm.org/docs/CommandGuide/llvm-cov.html

LLVM branch regions associate a source range with separate true and false counters. `llvm-cov export` publishes branches, functions, files and summaries in JSON, allowing repeated function/codegen records to be reconciled against one physical source identity while preserving the exported production-file denominator.
