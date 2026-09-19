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

fn named_step_section<'a>(job: &'a str, step_name: &str) -> &'a str {
    let marker = format!("\n      - name: {step_name}\n");
    let start = job
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI step {step_name}"));
    let body_start = start + marker.len();
    let remainder = &job[body_start..];
    let end = remainder
        .find("\n      - ")
        .map_or(job.len(), |offset| body_start + offset);
    &job[start..end]
}

fn executable_run_script(step: &str) -> &str {
    let run_marker = step
        .find("\n        run: |")
        .or_else(|| step.find("\n        run: >-"))
        .unwrap_or_else(|| panic!("attestation step must execute an inline run script"));
    let script_start = step[run_marker + 1..]
        .find('\n')
        .map(|offset| run_marker + 1 + offset + 1)
        .unwrap_or(step.len());
    &step[script_start..]
}

fn is_network_probe_command(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        " nc ",
        " ncat ",
        " curl ",
        "/dev/tcp/",
        " dig ",
        " getent ",
        " ping ",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
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
fn every_checkout_discards_persisted_credentials() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let lines: Vec<&str> = workflow.lines().collect();
    let mut checkout_count = 0;

    for (index, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with("- uses: actions/checkout@") {
            continue;
        }
        checkout_count += 1;
        let step_indent = leading_spaces(line);
        let step_body = lines[index + 1..].iter().take_while(|candidate| {
            candidate.trim().is_empty() || leading_spaces(candidate) > step_indent
        });
        assert!(
            step_body
                .clone()
                .any(|candidate| candidate.trim() == "persist-credentials: false"),
            "every checkout step must disable persisted Git credentials"
        );
    }

    assert!(
        checkout_count > 0,
        "CI must contain at least one checkout step"
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

#[test]
fn positive_lsm_runner_attests_private_network_denial_before_checkout() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let job = job_section(&workflow, "podman-e2e-positive-lsm");
    let checkout = job
        .find("- uses: actions/checkout@")
        .expect("positive LSM job must retain exact-head checkout");
    let gate_name = "Attest self-hosted runner LAN and host-service denial";
    let gate_marker = format!("- name: {gate_name}");
    let gate = job.find(&gate_marker).unwrap_or_else(|| {
        panic!(
            "positive LSM self-hosted runner must machine-attest LAN/host-service denial before checkout"
        )
    });

    assert!(
        gate < checkout,
        "self-hosted network-isolation attestation must run before repository checkout"
    );

    let gate_step = named_step_section(job, gate_name);
    assert!(
        !gate_step.contains("uses:"),
        "pre-checkout isolation attestation must execute directly rather than fetch another action before repository checkout"
    );
    let script = executable_run_script(gate_step);
    let active_lines: Vec<&str> = script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    assert!(
        active_lines.iter().any(|line| *line == "set -euo pipefail"),
        "attestation script must fail closed on shell errors and unset inputs"
    );
    assert!(
        active_lines
            .iter()
            .any(|line| line.starts_with("probe_forbidden_endpoint()")),
        "attestation script must centralize forbidden-endpoint probing in an executable helper"
    );
    assert!(
        active_lines
            .iter()
            .any(|line| is_network_probe_command(line)),
        "forbidden-endpoint helper must execute a concrete network/DNS probe rather than only print or describe evidence"
    );

    for required_scope in [
        "192.168.0.0/16",
        "10.0.0.0/8",
        "172.16.0.0/12",
        "6379",
        "5432",
        "DNS",
    ] {
        assert!(
            active_lines.iter().any(|line| {
                line.contains("probe_forbidden_endpoint")
                    && line.contains(required_scope)
                    && !line.starts_with("echo ")
                    && !line.starts_with("printf ")
            }),
            "pre-checkout attestation must pass {required_scope} through the executable forbidden-endpoint probe instead of mentioning it only in comments, output, or configuration text"
        );
    }

    assert!(
        active_lines.iter().any(|line| {
            let lower = line.to_ascii_lowercase();
            lower.starts_with("if ") && is_network_probe_command(line)
        }),
        "forbidden-endpoint helper must branch directly on an actual network/DNS probe result"
    );
    assert!(
        active_lines.iter().any(|line| line.contains("exit 1")),
        "a reachable forbidden endpoint must fail the attestation gate nonzero"
    );
    assert!(
        active_lines.iter().any(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("evidence") && lower.contains(".json")
        }),
        "attestation gate must emit machine-readable JSON evidence that binds probes to observed denial results"
    );
}
