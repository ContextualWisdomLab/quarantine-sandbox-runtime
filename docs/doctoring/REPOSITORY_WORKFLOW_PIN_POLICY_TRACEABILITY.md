# Repository workflow pin policy traceability

## Decision status

Proposed security-policy repair. The original hardened RED executed causally, the same-repository `$/...` false-positive RED executed causally, and a later exact-source review found a second false-negative: the line-oriented matcher did not inspect YAML flow mappings that contain `uses`. Current minimum implementation is a candidate only; every moved exact head still requires fresh CI/security/review evidence.

## Problem and live authority

Root PR #1 keeps `scripts/validate_repository.py` as the repository-local policy gate. The repository-local validator is defense in depth for checked-in workflows; organization/reusable-workflow policy remains owned by `ContextualWisdomLab/.github`.

The first review found that the original validator read only `.github/workflows/ci.yml`, missed normal sequence-step `- uses: ...` syntax, and raised when the workflow surface was absent. Those findings produced the initial multi-workflow RED and repair.

The next review found an opposite error: GitHub's same-repository `$/...` references are bound to the running workflow commit and cannot carry an `@ref`, but the first minimum repair classified them as unpinned external dependencies. That RED executed before the bounded `$/` exemption was added.

Current-root-restacked predecessor `c5f365da755ec7ff003034c5ab9f17af4a7d2b5e` still used `^\s*(?:-\s*)?uses:`. YAML 1.2.2 defines a flow mapping written with `{ ... }` as the same mapping data model expressed in flow style, and GitHub defines workflow files as YAML with `jobs.<job_id>.steps[*].uses` carrying action dependencies. A valid step such as `- { name: checkout, uses: actions/checkout@v4 }` therefore contains the same mutable external dependency but bypasses the anchored matcher entirely. Review `5139715214` records this false-negative.

## Executed predecessor REDs

Initial test-bearing commit `f67bcb452f8002ed7eda13eb8e1988c0b1e035ed` added multi-workflow/missing-workflow fixtures. CI-harness `1de70ec9ee07a09b4e1964052746ffbd99646ae4` made them executable in verify. Hardened RED `54138900896cc7aa319fe2066c62ce8209f87ca3` added the primary `ci.yml` `- uses:` case. Exact `54299dd50748cb5237530299ed6e1fa6e3845781`, native CI `34150580702`, verify `101831850483` failed at the dedicated repository-policy tests for those intended causes.

Review `5134607273` then added same-repository positive controls. Exact `8f33668cc7d3481070474220522b67858c694aca`, native CI `34152437247`, verify `101837316739` reproduced that focused RED: `$/.github/actions/runtime-policy` and `$/.github/workflows/reusable.yml` were rejected by the blanket external-reference rule while the other focused cases passed.

## Flow-mapping RED and minimum repair

Test-only commit `2712a12dda219d6350e05b678f076a4715c39b75` adds `test_unpinned_action_in_flow_style_step_fails`. The preceding validator's anchored regex does not match the fixture line even though a standards-conforming YAML parser resolves it to a step mapping whose `uses` value is `actions/checkout@v4`. This is the new causal contract defect; exact hosted execution is still required before the GitHub check itself is called RED evidence.

Minimum candidate `8f85baaed852b835174f6f2a4cff5473cb2bb908` removes the one-line matcher and adds a dependency-free lexical extractor scoped to workflow dependency keys. It recognizes block mappings and YAML flow mappings, including quoted `uses` keys, preserves `$/` self-repository semantics, and skips indented block-scalar bodies so shell text such as `printf 'uses: actions/checkout@v4'` is not mistaken for a dependency.

Test hardening `1c97a75969fc122cdecb1234e515e659bcfbcbf0` adds a quoted flow-key negative case, a full-SHA flow-style positive control, and a block-scalar script control. The repair does not add PyYAML or another mutable validator dependency; repository policy remains executable with the Python standard library already used by the gate.

## Alternatives considered

**Scan only `ci.yml`.** Rejected because repository policy must cover the workflow set, not one historical filename.

**Keep the anchored line regex and document block-style YAML only.** Rejected because the policy claim is about immutable external dependencies, while GitHub consumes YAML semantics. An equivalent mapping spelling must not silently change the security result.

**Add PyYAML without an explicit pinned toolchain contract.** Rejected because that would make a security admission gate depend on an undeclared interpreter package and would move the problem into environment reproducibility.

**Search every raw line for `uses:`.** Rejected because block-scalar `run: |` bodies can legitimately contain arbitrary text. A security validator that turns shell/script text into dependency declarations creates false positives and incentives to bypass the gate.

**Require `@<sha>` even for `$/*`.** Rejected because GitHub defines `$/` as the running repository commit and forbids an `@ref` suffix.

**Allow `./` and `$/*` interchangeably.** Rejected for this slice. GitHub recommends `$/` because it resolves directly against the running workflow repository/commit; `./` resolves against the checked-out workspace and has different trust semantics.

**Allow tags from trusted publishers.** Rejected because the repository contract intentionally requires immutable full commit identities for external dependencies.

## Security rationale

GitHub documents commit SHA as the safest external action reference and defines `$/` as a same-repository reference resolved at the running workflow commit. GitHub also defines workflows as YAML. YAML 1.2.2 states that flow mappings written with curly braces are a representation of mappings, not a different data type. Repository pin admission therefore has to operate on the dependency key independent of block-versus-flow spelling while keeping script scalar content out of the dependency surface.

## References

GitHub. (2026). *Secure use reference*. GitHub Docs. https://docs.github.com/en/actions/reference/security/secure-use

GitHub. (2026). *Workflow syntax for GitHub Actions*. GitHub Docs. https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax

GitHub. (2026). *Managing GitHub Actions settings for a repository*. GitHub Docs. https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository

Open Source Security Foundation. (n.d.). *Scorecard FAQ: Pinned dependencies*. https://github.com/ossf/scorecard/blob/main/docs/faq.md

YAML Language Development Team. (2021). *YAML Ain't Markup Language (YAML™) version 1.2.2*. https://yaml.org/spec/1.2.2/

## Completion boundary

Current #92 exact head is `1c97a75969fc122cdecb1234e515e659bcfbcbf0` on root `5c6a44bb2b35eb17d0315d72db242f4488c3c426`. The flow-mapping candidate and focused local lexical checks are not exact-head GitHub GREEN. Fresh repository-policy tests, fmt/tests/Clippy/rustdoc, complete owned-production coverage, review/thread/security gates, positive effective-LSM, protected integration, SBOM/provenance, reproducibility, rollback, and immutable release evidence remain independent requirements. Predecessor checks never transfer across head movement.
