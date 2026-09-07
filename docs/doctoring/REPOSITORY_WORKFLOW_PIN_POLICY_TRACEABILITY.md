# Repository workflow pin policy traceability

## Decision status

Proposed security-policy repair. Production/runtime behavior is unchanged on this branch. The current slice is RED-only until GitHub-hosted execution demonstrates the repository validator's workflow action-discovery gaps on the exact head.

## Problem and live authority

Root PR #1 exact `78281e244c530dcafb3368b9f1d9896e846206a9` keeps `scripts/validate_repository.py` as the repository-local policy gate. Fresh exact-source review found two independent false-negative paths in its action-pin check.

First, the validator reads only `.github/workflows/ci.yml`; a second `.yml` or `.yaml` workflow is outside the scan. If `ci.yml` is absent, `read_text()` raises instead of returning a deterministic policy failure.

Second, the validator collects only lines for which `line.strip().startswith("uses:")`. The repository's real workflow steps use normal YAML sequence syntax such as `- uses: actions/checkout@<sha>`, and the RED fixtures intentionally use the same form. After stripping whitespace those lines begin with `- uses:`, so the current validator does not inspect even the existing `ci.yml` action references. Review `5134484233` records this broader false-negative.

The local policy therefore proves neither the repository workflow set nor the action references in the workflow it currently opens. A mutable tag or branch may be present in a normal workflow step while `python3 scripts/validate_repository.py` still returns success.

This is repository security-policy validation, not `sandbox_execution`, `application_service`, or `artifact_analysis` domain behavior. It must not move runtime authority into a generic helper or weaken the exact-SHA contract. Organization/reusable-workflow policy remains owned by `ContextualWisdomLab/.github`; this repository-local validator is defense in depth for the files actually present here.

## RED contract

Initial test-bearing commit `f67bcb452f8002ed7eda13eb8e1988c0b1e035ed` added `scripts/test_validate_repository.py`. CI-harness commit `1de70ec9ee07a09b4e1964052746ffbd99646ae4` invokes that test from the existing verify lane without adding a new action dependency. Hardened RED commit `54138900896cc7aa319fe2066c62ce8209f87ca3` adds a primary-`ci.yml` case so sequence-step parsing is independently observable from multi-file discovery.

The isolated fixture tests now require:

- an unpinned action written as normal `- uses:` syntax in primary `ci.yml` to fail policy validation;
- an unpinned action in a second `*.yml` workflow to fail;
- an unpinned action in a `*.yaml` workflow to fail;
- a missing workflow directory to return a deterministic nonzero policy result instead of raising an uncaught filesystem exception;
- multiple workflows whose actions are all full-length SHA pinned to remain valid.

Current production should fail the primary-`ci.yml`, second-`.yml`, `.yaml`, and missing-workflow requirements for their respective parsing/discovery/admission causes while the fully pinned control remains GREEN. No validator implementation change is authorized until this hardened RED executes causally.

## Minimum causal GREEN

After causal execution, change only repository workflow action discovery/admission:

1. enumerate regular `.yml` and `.yaml` files directly under `.github/workflows` in deterministic order;
2. return an explicit policy error if the workflow directory is missing, unreadable, or contains no workflow definitions;
3. recognize normal workflow step entries written as `- uses: owner/action@reference` and apply the existing exact 40-character lower-case hexadecimal SHA requirement to every discovered external action reference;
4. retain all current required-file, DDD, ADR, schema, placeholder, and database-name checks;
5. do not introduce tag/branch allowlists, mutable exceptions, or a special exemption for reusable/thin-caller workflows.

A full YAML-parser migration is not required for this bounded defect. The minimum repair must, however, parse the actual sequence-step shape the repository uses rather than preserving the current `startswith("uses:")` false negative. More complex YAML forms, if later admitted, require their own executable policy finding.

## Alternatives considered

**Scan only `ci.yml`.** Rejected because repository policy must cover the workflow set, not one historical filename.

**Keep `startswith("uses:")` and only add multi-file enumeration.** Rejected because the repository's real action steps begin with `- uses:`; discovery without recognizing the admitted syntax remains a false GREEN.

**Rely only on organization GitHub Actions policy.** Rejected as the sole repository check. Organization policy is valuable enforcement but does not make the checked-in repository validator truthful or portable to forks/local review.

**Allow tags from trusted publishers.** Rejected. The existing repository contract intentionally requires immutable commit identities, and changing that acceptance set is unrelated to workflow discovery.

**Silently accept no workflow directory.** Rejected. This repository requires CI/release evidence, so absence of the configured workflow surface is a policy failure rather than success.

## Security rationale

GitHub's current Secure use reference states that pinning an action to a full-length commit SHA is the only way to consume an action as an immutable release, and GitHub exposes repository/organization policy to require full-length SHA pins. OpenSSF Scorecard likewise recommends hash pinning rather than version/tag pinning because tags can be renamed or repointed. The local validator should enforce that immutable-reference property over every action reference in every workflow it claims to validate.

## References

GitHub. (2026). *Secure use reference*. GitHub Docs. https://docs.github.com/en/actions/reference/security/secure-use

GitHub. (2026). *Managing GitHub Actions settings for a repository*. GitHub Docs. https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository

Open Source Security Foundation. (n.d.). *Scorecard FAQ: Pinned dependencies*. https://github.com/ossf/scorecard/blob/main/docs/faq.md

## Completion boundary

A RED or GREEN in this focused policy test does not transfer root production coverage, security scanning, positive LSM, review, protected-integration, SBOM/provenance, reproducibility, rollback, or immutable-release evidence. The child must preserve current root ancestry non-force and reacquire all applicable exact-head gates after every movement.
