//! CI runner-selection and exact-head trigger regression contracts.

use std::fs;

fn section_end(workflow: &str, body_start: usize) -> usize {
    let remainder = &workflow[body_start..];
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n');
        let sibling_header = line_without_newline.starts_with("  ")
            && !line_without_newline.starts_with("   ")
            && !line_without_newline.trim().is_empty();
        let top_level = !line_without_newline.is_empty() && !line_without_newline.starts_with(' ');
        if sibling_header || top_level {
            return body_start + offset;
        }
        offset += line.len();
    }
    workflow.len()
}

fn job_section<'a>(workflow: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI job {job_name}"));
    let body_start = start + marker.len();
    let end = section_end(workflow, body_start);
    &workflow[start..end]
}

fn event_section<'a>(workflow: &'a str, event_name: &str) -> &'a str {
    let marker = format!("\n  {event_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI event {event_name}"));
    let body_start = start + marker.len();
    let end = section_end(workflow, body_start);
    &workflow[start..end]
}

#[test]
fn ci_delegates_to_the_immutable_central_self_hosted_workflow() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let job = job_section(&workflow, "ci");
    let prefix =
        "uses: ContextualWisdomLab/.github/.github/workflows/quarantine-sandbox-runtime-ci.yml@";
    let call = job
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(prefix))
        .expect("CI must call the central QSR workflow");
    assert_eq!(call.len(), 40, "central revision must be a full commit SHA");
    assert!(
        call.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_ne!(call, "0000000000000000000000000000000000000000");
    assert!(
        !workflow.contains("runs-on:"),
        "caller cannot choose a runner"
    );
    assert!(
        !workflow.contains("steps:"),
        "execution belongs to the pinned producer"
    );
}

#[test]
fn central_caller_does_not_inherit_credentials_or_override_execution() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let job = job_section(&workflow, "ci");
    assert!(workflow.contains("contents: read"));
    for forbidden in [
        "secrets:",
        "with:",
        "env:",
        "steps:",
        "if:",
        "continue-on-error:",
    ] {
        assert!(
            !job.contains(forbidden),
            "central caller must not contain {forbidden}"
        );
    }
    assert!(
        !workflow.contains("actions/checkout@"),
        "credential-free exact checkout is implemented and verified in the pinned producer"
    );
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
