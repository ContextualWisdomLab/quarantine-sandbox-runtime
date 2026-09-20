# Release-evidence gap-owner repair

## Scope

PR #10 owns executable evidence for the first immutable commercial release above the effective-isolation parent. Its domain truth includes the release workflow/runbook, release-evidence schema, package/attestation/delivery checks, protected-source validation, SBOM/provenance/checksum expectations, and the rule that a PR-head result is not a substitute for evidence on the exact protected integration SHA.

It does not own the repository-wide `docs/product-technical-gap-baseline.md`; that ledger remains #121 authority. Effective-isolation truth remains #9 and lease ownership/idempotency remains #6.

## Retained causal evidence

The release line remains unreleased. Protected/default `develop` is repository authority, and release preflight must resolve the actual protected source rather than hard-code stale `main` assumptions. Root issue #24 is complementary: after normal integration, native push-triggered CI must materialize on the exact protected `develop` integration SHA before that SHA can become release authority.

Historical #10 exact `5feaa1b4df77946db76f2d735c910a0ee34a37ad` preserved the release workflow, runbook, evidence schema, attestation/delivery contracts, and protected-source RED above then-current #9. Those observations do not transfer after parent movement.

## Single-writer and parent-adoption decision

Review `5230238840` found that #10 still carried the global Gap ledger and had not adopted parent #9 exact `156959e92d5fa80059ba379c7c46670f259283de`. The repair is migration-first: preserve release-specific history here, then adopt #9 by ordinary non-force ancestry while inheriting the parent owner-local doctoring and restored global-Gap state.

No release workflow semantics, schema contract, production Rust, test assertion, or release claim is weakened by this ownership repair. No force push or destructive rebase is permitted. The resulting exact #10 head must reacquire its own CI/security/coverage/release-contract evidence; immutable publication remains prohibited until one exact protected integrated head satisfies all release gates.

## 2026-09-20 exact-head CI RCA

CI run `35175389416` on predecessor head
`b26642bb86bafa8b7f2cfb0b25c0f13b4aa5a6c4` exposed two distinct evidence classes.

- The deterministic release-owner RED was
  `tests/release_delivery_contract.rs::repository_exposes_fail_closed_release_delivery_contract`:
  the checked-in contract requires native CI for both live `develop` integration and stable
  `main`, while `.github/workflows/ci.yml` listened only to `develop` pushes. Commit
  `90b491cf0c9274abae8591ae52785b6ab914bd11` adds `main` to the existing push branch
  filter without changing permissions, required jobs, runner identities, or test thresholds.
- The branch-coverage shard alone failed
  `confined_apparmor_mode_suffix_is_the_same_effective_profile`, while the same test passed
  on the exact-head ordinary verify shard. That inconsistent same-head result is retained as
  a possible parallel fixture/runtime race, not reclassified as GREEN and not used to weaken
  the LSM contract. Fresh exact-head evidence after the deterministic cause change must show
  whether it recurs before any fixture or runtime repair is justified.

The positive-LSM job was cancelled after the aggregate failure and therefore supplies no
acceptance evidence. The PR remains Draft; neither this PR-head result nor predecessor results
authorize immutable publication.

