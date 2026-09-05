//! CI contract regressions for real rootless LSM acceptance.

use std::fs;

fn job_section<'a>(workflow: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI job {job_name}"));
    let body_start = start + marker.len();
    let remainder = &workflow[body_start..];
    let end = remainder
        .match_indices("\n  ")
        .find_map(|(offset, _)| {
            let next_indent_byte = remainder.as_bytes().get(offset + 3).copied();
            (next_indent_byte.is_some() && next_indent_byte != Some(b' '))
                .then_some(body_start + offset)
        })
        .unwrap_or(workflow.len());
    &workflow[start..end]
}

#[test]
fn ci_separates_rootless_apparmor_negative_evidence_from_positive_lsm_acceptance() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");

    let negative = job_section(&workflow, "podman-e2e-negative-rootless-apparmor");
    assert!(negative.contains("runs-on: ubuntu-24.04"));
    assert!(negative.contains("podman info --format json"));
    assert!(negative.contains("apparmorEnabled"));
    assert!(negative.contains("selinuxEnabled"));
    assert!(negative.contains("rootless_podman_rejects_unavailable_effective_lsm_and_cleans_up"));
    assert!(negative.contains("if: always()"));
    assert!(negative.contains("qsr-net-"));

    let positive = job_section(&workflow, "podman-e2e-positive-lsm");
    assert!(positive.contains("self-hosted"));
    assert!(positive.contains("cwl-hostile-workload"));
    assert!(positive.contains("selinux"));
    assert!(positive.contains("podman info --format json"));
    assert!(positive.contains("selinuxEnabled"));
    assert!(positive.contains("rootless_podman_effective_isolation_and_cleanup"));
    assert!(positive.contains("RUNNER_NAME"));
    assert!(positive.contains("github.event_name == 'push'"));
    assert!(
        positive.contains("github.event.pull_request.head.repo.full_name == github.repository")
    );
    assert!(positive.contains("if: always()"));
    assert!(positive.contains("qsr-net-"));
    assert!(!positive.contains("runs-on: ubuntu-24.04"));
    assert!(!positive.contains("runs-on: ubuntu-latest"));
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
