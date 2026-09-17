//! Repository-level regression for exact-head local CI evidence.
//!
//! Documentation-only head movement still changes the PR head identity. The
//! local CI workflow therefore must not suppress `pull_request`/`push` runs
//! with `paths-ignore`, because predecessor-head product evidence is not
//! transferable to the new exact head.

#[test]
fn local_ci_does_not_skip_documentation_only_head_changes() {
    let workflow = include_str!("../.github/workflows/ci.yml");

    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("push:"));
    assert!(
        !workflow.contains("paths-ignore:"),
        "local CI must materialize for every changed PR head; queue pressure is a control-plane concern, not permission to reuse predecessor evidence"
    );
}
