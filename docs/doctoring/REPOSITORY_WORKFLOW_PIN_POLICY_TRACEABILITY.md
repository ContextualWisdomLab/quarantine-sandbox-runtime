# Repository workflow pin policy traceability

## Decision status

Proposed security-policy repair. Production/runtime behavior is unchanged on this branch. The current slice is RED-only until GitHub-hosted execution demonstrates the repository validator's workflow-discovery gap on the exact head.

## Problem and live authority

Root PR #1 exact `78281e244c530dcafb3368b9f1d9896e846206a9` keeps `scripts/validate_repository.py` as the repository-local policy gate. On that source, action pin validation reads only `.github/workflows/ci.yml`, collects its `uses:` lines, and requires a 40-character lower-case hexadecimal reference. A second `.yml` or `.yaml` workflow is outside that scan. If `ci.yml` is absent, `read_text()` raises rather than returning a deterministic policy failure.

The policy therefore proves one filename, not the repository workflow set. A later workflow can introduce a mutable tag or branch while `python3 scripts/validate_repository.py` still returns success.

This is repository security-policy validation, not `sandbox_execution`, `application_service`, or `artifact_analysis` domain behavior. It must not move runtime authority into a generic helper or weaken the existing exact-SHA rule. Organization/reusable-workflow policy remains owned by `ContextualWisdomLab/.github`; this repository-local validator is defense in depth for the files actually present here.

## RED contract

Test-bearing commit `f67bcb452f8002ed7eda13eb8e1988c0b1e035ed` adds `scripts/test_validate_repository.py`. CI-harness commit `1de70ec9ee07a09b4e1964052746ffbd99646ae4` invokes that test from the existing verify lane without adding a new action dependency.

The isolated fixture tests require:

- an unpinned action in a second `*.yml` workflow to fail policy validation;
- an unpinned action in a `*.yaml` workflow to fail policy validation;
- a missing workflow directory to return a deterministic nonzero policy result instead of raising an uncaught filesystem exception;
- multiple workflows whose actions are all full-length SHA pinned to remain valid.

The current validator should fail the first three requirements for independent reasons while the positive control remains GREEN. No production/policy implementation change is authorized until that causal RED executes.

## Minimum causal GREEN

After causal execution, change only repository workflow discovery/admission:

1. enumerate regular `.yml` and `.yaml` files directly under `.github/workflows` in deterministic order;
2. return an explicit policy error if the workflow directory is missing, unreadable, or contains no workflow definitions;
3. inspect every discovered workflow's `uses:` entries with the existing exact 40-character lower-case SHA requirement;
4. retain all current required-file, DDD, ADR, schema, placeholder, and database-name checks;
5. do not introduce tag/branch allowlists, mutable exceptions, or a special exemption for reusable/thin-caller workflows.

A parser refactor is not required for this bounded defect. If future YAML constructs make line-oriented `uses:` discovery insufficient, that becomes a separate executable policy finding rather than scope creep in this first repair.

## Alternatives considered

**Scan only `ci.yml`.** Rejected because repository policy must cover the workflow set, not one historical filename.

**Rely only on organization GitHub Actions policy.** Rejected as the sole repository check. Organization policy is valuable enforcement but does not make the checked-in repository validator truthful or portable to forks/local review.

**Allow tags from trusted publishers.** Rejected. The existing repository contract intentionally requires immutable commit identities, and changing that acceptance set is unrelated to workflow discovery.

**Silently accept no workflow directory.** Rejected. This repository requires CI/release evidence, so absence of the configured workflow surface is a policy failure rather than success.

## Security rationale

GitHub's current Secure use reference states that pinning an action to a full-length commit SHA is the only way to consume an action as an immutable release, and GitHub exposes repository/organization policy to require full-length SHA pins. OpenSSF Scorecard likewise recommends hash pinning rather than version/tag pinning because tags can be renamed or repointed. The local validator should enforce that same immutable-reference property over every repository workflow it claims to validate.

## References

GitHub. (2026). *Secure use reference*. GitHub Docs. https://docs.github.com/en/actions/reference/security/secure-use

GitHub. (2026). *Managing GitHub Actions settings for a repository*. GitHub Docs. https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository

Open Source Security Foundation. (n.d.). *Scorecard FAQ: Pinned dependencies*. https://github.com/ossf/scorecard/blob/main/docs/faq.md

## Completion boundary

A RED or GREEN in this focused policy test does not transfer root production coverage, security scanning, positive LSM, review, protected-integration, SBOM/provenance, reproducibility, rollback, or immutable-release evidence. The child must preserve current root ancestry non-force and reacquire all applicable exact-head gates after every movement.
