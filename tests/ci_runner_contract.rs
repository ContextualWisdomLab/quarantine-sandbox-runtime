//! CI runner-selection regression contracts.

use std::fs;

fn job_section<'a>(workflow: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI job {job_name}"));
    let body_start = start + marker.len();
    let remainder = &workflow[body_start..];
    let end = remainder
        .find("\n  ")
        .map_or(workflow.len(), |offset| body_start + offset);
    &workflow[start..end]
}

#[test]
fn ordinary_hosted_ci_uses_explicit_supported_runner_image() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");

    for job_name in ["verify", "coverage", "branch-coverage"] {
        let job = job_section(&workflow, job_name);
        assert!(
            job.contains("runs-on: ubuntu-24.04"),
            "{job_name} must use the explicit supported hosted runner image"
        );
        assert!(
            !job.contains("runs-on: ubuntu-latest"),
            "{job_name} must not depend on the floating hosted runner selector"
        );
    }
}

#[test]
fn ci_cancels_only_superseded_heads_of_the_same_pull_request() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");

    for required in [
        "github.workflow",
        "github.repository",
        "github.event.pull_request.number",
        "github.run_id",
        "cancel-in-progress: true",
    ] {
        assert!(
            workflow.contains(required),
            "CI concurrency must contain {required}"
        );
    }
    assert!(
        !workflow.contains("github.ref }}"),
        "non-PR runs must not share a ref-scoped cancellation group"
    );
}

#[test]
fn ci_runs_on_integrated_protected_develop_head() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");

    assert!(
        workflow.contains("push:\n    branches: [develop]"),
        "native CI must run after integration into the protected default branch develop"
    );
    assert!(
        !workflow.contains("push:\n    branches: [main]"),
        "native CI must not retain the stale main-only push trigger"
    );
}
