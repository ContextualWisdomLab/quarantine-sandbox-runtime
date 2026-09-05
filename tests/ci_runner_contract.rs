//! CI runner-selection and exact-head trigger regression contracts.

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

fn event_section<'a>(workflow: &'a str, event_name: &str) -> &'a str {
    let marker = format!("\n  {event_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI event {event_name}"));
    let body_start = start + marker.len();
    let remainder = &workflow[body_start..];
    let end = remainder
        .match_indices('\n')
        .find_map(|(offset, _)| {
            let next_line = &remainder[offset + 1..];
            let line = next_line.lines().next().unwrap_or_default();
            let next_event = line.starts_with("  ") && !line.starts_with("    ");
            let next_top_level = !line.is_empty() && !line.starts_with(' ');
            (next_event || next_top_level).then_some(body_start + offset)
        })
        .unwrap_or(workflow.len());
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
fn ci_runs_on_every_integrated_protected_develop_head() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let push = event_section(&workflow, "push");

    assert!(
        push.contains("branches: [develop]"),
        "native CI must run after integration into the protected default branch develop"
    );
    assert!(
        !push.contains("branches: [main]"),
        "native CI must not retain the stale main-only push trigger"
    );
    assert!(
        !push.lines().any(|line| {
            let key = line.trim_start();
            key.starts_with("paths:") || key.starts_with("paths-ignore:")
        }),
        "protected-develop CI must not exclude documentation-only or other integrated heads with push path filters; exact integrated SHA evidence is required regardless of changed path"
    );
}

#[test]
fn pull_request_ci_does_not_skip_documentation_only_heads() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let pull_request = event_section(&workflow, "pull_request");

    assert!(
        !pull_request.lines().any(|line| {
            let key = line.trim_start();
            key.starts_with("paths:") || key.starts_with("paths-ignore:")
        }),
        "PR CI must materialize for documentation-only head movement; predecessor exact-head evidence is not transferable"
    );
}
