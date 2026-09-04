//! Repository-level release delivery contract.
//!
//! These tests intentionally inspect checked-in release automation rather than
//! executing publication. They keep release evidence fail-closed and reviewable.

use std::{fs, path::Path};

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn job_section<'a>(workflow: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing release job {job_name}"));
    let body_start = start + marker.len();
    let remainder = &workflow[body_start..];
    let end = remainder
        .lines()
        .scan(body_start, |offset, line| {
            let line_start = *offset;
            *offset += line.len() + 1;
            Some((line_start, line))
        })
        .find(|(_, line)| {
            line.starts_with("  ") && !line.starts_with("    ") && line.trim_end().ends_with(':')
        })
        .map_or(workflow.len(), |(offset, _)| offset);
    &workflow[start..end]
}

#[test]
fn repository_exposes_fail_closed_release_delivery_contract() {
    let root = repository_root();
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .expect("CI workflow must be readable");
    assert!(
        ci.contains("branches: [develop, main]"),
        "repository CI must cover both live develop integration and stable main"
    );

    let release_runbook = root.join("RELEASE.md");
    assert!(
        release_runbook.is_file(),
        "release delivery requires a checked-in RELEASE.md runbook"
    );

    let release_workflow = root.join(".github/workflows/release.yml");
    let release = fs::read_to_string(&release_workflow)
        .expect("release delivery requires .github/workflows/release.yml");

    for required in [
        "tags:",
        "v*",
        "refs/remotes/origin/main",
        "cargo package --locked",
        "cargo llvm-cov",
        "--branch",
        "cwl-hostile-workload",
        "selinux",
        "podman info",
        "rootless_podman_effective_isolation_and_cleanup",
        "spdx-json@3.0",
        "SHA256SUMS",
        "actions/attest@",
        "gh release create",
    ] {
        assert!(
            release.contains(required),
            "release workflow is missing required fail-closed evidence token: {required}"
        );
    }
}

#[test]
fn release_hosted_jobs_use_explicit_supported_runner_image() {
    let release = fs::read_to_string(repository_root().join(".github/workflows/release.yml"))
        .expect("release workflow must be readable");

    for job_name in [
        "preflight",
        "statement-coverage",
        "branch-coverage",
        "package-evidence",
        "attest-and-release",
    ] {
        let job = job_section(&release, job_name);
        assert!(
            job.contains("runs-on: ubuntu-24.04"),
            "{job_name} must use the explicit supported hosted runner image"
        );
        assert!(
            !job.contains("runs-on: ubuntu-latest"),
            "{job_name} must not depend on the floating hosted runner selector"
        );
    }

    let hostile = job_section(&release, "hostile-runtime-e2e");
    assert!(hostile.contains("self-hosted"));
    assert!(hostile.contains("cwl-hostile-workload"));
    assert!(hostile.contains("selinux"));
}
