//! Regression for protected production release-source authority (#140).
//!
//! `develop` is development integration/default, not production authority.
//! A tag must equal the current protected production `main` head. Missing,
//! stale or unprotected `main` must fail closed; default-branch status is not
//! a substitute. These source checks do not authorize publication or prove
//! live branch protection, immutable-release policy or signed attestations.

fn has_protected_production_source_contract(workflow: &str) -> bool {
    let Some(preflight) = workflow.split("\n  preflight:\n").nth(1) else {
        return false;
    };
    let Some(source_step) = preflight
        .split("      - name: Verify exact protected-main release source\n")
        .nth(1)
        .and_then(|step| step.split("      - name:").next())
    else {
        return false;
    };
    [
        r#"test "$(git rev-parse HEAD)" = "$EXPECTED_SHA""#,
        "git fetch --no-tags --depth=1 origin main",
        r#"test "$(git rev-parse refs/remotes/origin/main)" = "$EXPECTED_SHA""#,
        r#"test "$(gh api "repos/${GITHUB_REPOSITORY}/branches/main" --jq '.protected')" = "true""#,
    ]
    .iter()
    .all(|required| source_step.lines().any(|line| line.trim() == *required))
}

#[test]
fn release_preflight_requires_exact_protected_production_main() {
    assert!(
        has_protected_production_source_contract(include_str!("../.github/workflows/release.yml")),
        "release preflight must bind the exact tag checkout to the current protected production main head"
    );
}

#[test]
fn release_preflight_rejects_develop_as_publication_authority() {
    let workflow = include_str!("../.github/workflows/release.yml");
    let develop = workflow
        .replace("origin main", "origin develop")
        .replace("origin/main", "origin/develop")
        .replace("branches/main", "branches/develop");
    assert!(!has_protected_production_source_contract(&develop));
}

#[test]
fn release_preflight_rejects_missing_protection_proof() {
    let workflow = include_str!("../.github/workflows/release.yml");
    let unprotected = workflow.replace(
        r#"test "$(gh api "repos/${GITHUB_REPOSITORY}/branches/main" --jq '.protected')" = "true""#,
        "true",
    );
    assert!(!has_protected_production_source_contract(&unprotected));
}

#[test]
fn release_preflight_rejects_missing_exact_production_head_proof() {
    let workflow = include_str!("../.github/workflows/release.yml");
    let stale = workflow.replace(
        r#"test "$(git rev-parse refs/remotes/origin/main)" = "$EXPECTED_SHA""#,
        "true",
    );
    assert!(!has_protected_production_source_contract(&stale));
}
