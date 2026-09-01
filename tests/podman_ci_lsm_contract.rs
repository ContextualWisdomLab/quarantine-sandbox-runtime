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
        .find("\n  ")
        .map_or(workflow.len(), |offset| body_start + offset);
    &workflow[start..end]
}

#[test]
fn ci_separates_rootless_apparmor_negative_evidence_from_positive_lsm_acceptance() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");

    let negative = job_section(&workflow, "podman-e2e-negative-rootless-apparmor");
    assert!(negative.contains("runs-on: ubuntu-24.04"));
    assert!(negative.contains("ApparmorEnabled"));
    assert!(negative.contains("SelinuxEnabled"));
    assert!(negative.contains("rootless_podman_rejects_unavailable_effective_lsm_and_cleans_up"));
    assert!(negative.contains("if: always()"));
    assert!(negative.contains("qsr-net-"));

    let positive = job_section(&workflow, "podman-e2e-positive-lsm");
    assert!(positive.contains("self-hosted"));
    assert!(positive.contains("cwl-hostile-workload"));
    assert!(positive.contains("selinux"));
    assert!(positive.contains("SelinuxEnabled"));
    assert!(positive.contains("rootless_podman_effective_isolation_and_cleanup"));
    assert!(positive.contains("RUNNER_NAME"));
    assert!(positive.contains("github.event_name == 'push'"));
    assert!(positive.contains("github.event.pull_request.head.repo.full_name == github.repository"));
    assert!(positive.contains("if: always()"));
    assert!(positive.contains("qsr-net-"));
    assert!(!positive.contains("runs-on: ubuntu-24.04"));
    assert!(!positive.contains("runs-on: ubuntu-latest"));
}
