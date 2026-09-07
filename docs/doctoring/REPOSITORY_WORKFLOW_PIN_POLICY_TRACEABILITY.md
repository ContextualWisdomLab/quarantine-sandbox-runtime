# Repository workflow pin policy traceability

## Decision status

Proposed security-policy repair. The original hardened RED executed causally, but exact-source review of the first minimum repair found a false-positive boundary for GitHub's same-repository, same-commit `$/...` references. Production/policy commit `eeec132cc61fbc86a07c09f75be828afc4d566c3` therefore remains an incomplete candidate rather than GREEN.

## Problem and live authority

Root PR #1 exact `78281e244c530dcafb3368b9f1d9896e846206a9` keeps `scripts/validate_repository.py` as the repository-local policy gate. Exact-source review found two original false-negative paths in its action-pin check.

First, the validator read only `.github/workflows/ci.yml`; a second `.yml` or `.yaml` workflow was outside the scan. If `ci.yml` was absent, `read_text()` raised instead of returning a deterministic policy failure.

Second, it collected only lines for which `line.strip().startswith("uses:")`. The repository's real workflow steps use normal YAML sequence syntax such as `- uses: actions/checkout@<sha>`. After stripping whitespace those lines still begin with `- uses:`, so the validator did not inspect even the existing `ci.yml` action references. Review `5134484233` records this broader false-negative.

This is repository security-policy validation, not `sandbox_execution`, `application_service`, or `artifact_analysis` domain behavior. Organization/reusable-workflow policy remains owned by `ContextualWisdomLab/.github`; this repository-local validator is defense in depth for checked-in workflows.

## Executed RED

Initial test-bearing commit `f67bcb452f8002ed7eda13eb8e1988c0b1e035ed` added multi-workflow/missing-workflow fixtures. CI-harness `1de70ec9ee07a09b4e1964052746ffbd99646ae4` made them executable in verify. Hardened RED `54138900896cc7aa319fe2066c62ce8209f87ca3` added a primary-`ci.yml` case using the repository's admitted `- uses:` syntax. Gap-ledger head `54299dd50748cb5237530299ed6e1fa6e3845781` preserved zero production change.

Native CI `34150580702` on exact `54299dd...` executed the RED. Verify job `101831850483` passed exact checkout, dependency lock, and the existing repository-policy command, then failed at `Test repository policy validator`. A local exact-source reproduction separates the four intended failing cases: primary `ci.yml` `- uses:` was accepted, unpinned second `.yml` and `.yaml` workflows were accepted, and a missing workflow directory raised `FileNotFoundError`; the fully SHA-pinned multi-workflow control remained valid. The local reproduction is diagnostic detail; the GitHub job is the causal gate.

## First minimum repair

Production/policy commit `eeec132cc61fbc86a07c09f75be828afc4d566c3` changes only workflow action discovery/admission:

1. enumerate regular direct `.yml` and `.yaml` files under `.github/workflows` in deterministic order;
2. convert an unreadable/missing/empty workflow surface into explicit policy errors;
3. recognize both sequence-step `- uses: ...` and job-level `uses: ...` forms through one bounded line matcher;
4. apply the pre-existing exact 40-character lowercase hexadecimal SHA requirement to every discovered `uses` target;
5. retain every existing required-file, DDD, ADR, schema, placeholder, and database-name check.

The original five focused cases pass locally after this commit. That does not make the repair complete because the blanket fourth rule also classifies same-repository commit-bound references as external mutable dependencies.

## Review repair: same-repository commit-bound references

Review `5134607273` found the first repair over-broad. GitHub's current workflow syntax defines `$/path/to/action` and `$/.github/workflows/{filename}` as references to the same repository at the exact commit executing the workflow. An `@ref` suffix is forbidden for this syntax. The current validator nevertheless captures the target and rejects it at `"@" not in uses_target` as "unpinned".

Test-only commit `c4173ec8211debaaa753de3b027e677c2b73dbf4` adds two positive controls: a same-repository action referenced as `$/.github/actions/runtime-policy`, and a same-repository reusable workflow referenced as `$/.github/workflows/reusable.yml`. Both must remain admissible because their identity is already bound to the running workflow commit. External `owner/repository[/path]@reference` dependencies remain subject to the exact 40-character lowercase hexadecimal SHA rule; no tag, branch, or mutable external exception is introduced.

This RED is checked in but is not yet causal execution evidence. The current production validator is intentionally unchanged after the finding. The minimum follow-up GREEN, only after exact-head execution reaches the focused test for this cause, is to recognize the bounded `$/` self-repository form before applying the external-reference SHA rule. The weaker workspace-relative `./` form is not added by this slice.

## Alternatives considered

**Scan only `ci.yml`.** Rejected because repository policy must cover the workflow set, not one historical filename.

**Add multi-file enumeration but keep `startswith("uses:")`.** Rejected because the repository's admitted sequence-step syntax would remain unvalidated.

**Require `@<sha>` even for `$/*`.** Rejected because GitHub defines `$/` as the running repository commit and explicitly forbids an `@ref` suffix. The resulting policy would reject the platform's commit-bound self-reference form rather than strengthen it.

**Allow `./` and `$/*` interchangeably.** Rejected for this focused repair. GitHub recommends `$/` for same-repository actions because it binds directly to the workflow commit without depending on a checked-out mutable workspace. A separate requirement can evaluate `./` if a genuine compatibility case appears.

**Rely only on organization GitHub Actions policy.** Rejected as the sole repository check; it does not make the checked-in validator truthful or portable to forks/local review.

**Allow tags from trusted publishers.** Rejected because the existing local contract intentionally requires immutable commit identities for external dependencies.

**Silently accept no workflow directory.** Rejected because this repository requires CI/release evidence.

## Security rationale

GitHub's Secure use reference states that pinning an external action to a full-length commit SHA is the only way to consume that external action as an immutable release. GitHub's workflow syntax separately defines `$/` self-repository references as resolving to the exact running workflow commit, with no `@ref` suffix. The validator must preserve both properties: external dependencies are full-SHA pinned, while commit-bound self-references are not falsely rejected.

## References

GitHub. (2026). *Secure use reference*. GitHub Docs. https://docs.github.com/en/actions/reference/security/secure-use

GitHub. (2026). *Workflow syntax for GitHub Actions*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax

GitHub. (2026). *Managing GitHub Actions settings for a repository*. GitHub Docs. https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository

Open Source Security Foundation. (n.d.). *Scorecard FAQ: Pinned dependencies*. https://github.com/ossf/scorecard/blob/main/docs/faq.md

## Completion boundary

The executed original RED, first minimum candidate, and checked-in self-reference RED do not transfer root production coverage, security scanning, positive LSM, review, protected-integration, SBOM/provenance, reproducibility, rollback, or immutable-release evidence. Every head movement requires fresh exact-head gates.
