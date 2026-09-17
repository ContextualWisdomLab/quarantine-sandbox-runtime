//! Regression for immutable release-source authority.
//!
//! The repository default/protected integration branch is `develop`; the historical
//! `main` branch is not protected. A tag workflow must therefore resolve the live
//! repository default branch and prove that the tag commit is exactly that protected
//! branch head instead of hard-coding `main`.

#[test]
fn release_preflight_uses_live_protected_default_branch_not_historical_main() {
    let workflow = include_str!("../.github/workflows/release.yml");

    assert!(
        !workflow.contains("refs/remotes/origin/main"),
        "release preflight must not bind a tag to the historical unprotected main branch"
    );
    assert!(
        !workflow.contains("repos/${GITHUB_REPOSITORY}/branches/main"),
        "release preflight must not treat historical main as the protected release authority"
    );
    assert!(
        workflow.contains("default_branch"),
        "release preflight must resolve the repository default branch as release authority"
    );
    assert!(
        workflow.contains(".protected"),
        "release preflight must still prove that the resolved release branch is protected"
    );
}
