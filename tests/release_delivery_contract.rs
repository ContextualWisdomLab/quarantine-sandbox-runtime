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
        .match_indices("\n  ")
        .find_map(|(offset, _)| {
            let candidate = &remainder[offset + 3..];
            let line = candidate.lines().next()?;
            (line.ends_with(':') && !line.starts_with(' ')).then_some(body_start + offset)
        })
        .unwrap_or(workflow.len());
    &workflow[start..end]
}

fn assignment_line<'a>(script: &'a str, variable_name: &str) -> &'a str {
    let prefix = format!("{variable_name}=");
    script
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("missing {variable_name} assignment in release preflight"))
}

#[test]
fn repository_exposes_fail_closed_release_delivery_contract() {
    let root = repository_root();
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml"))
        .expect("CI workflow must be readable");
    assert!(
        ci.contains("branches: [develop]"),
        "repository CI must cover the protected default develop integration branch"
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
fn release_preflight_binds_source_to_live_default_branch() {
    let release = fs::read_to_string(repository_root().join(".github/workflows/release.yml"))
        .expect("release workflow must be readable");
    let preflight = job_section(&release, "preflight");

    assert!(
        preflight.contains(".default_branch"),
        "release preflight must read the repository's live default_branch authority"
    );
    assert!(
        preflight.contains("default_branch="),
        "release preflight must retain the live default branch as an explicit shell value"
    );
    assert!(
        preflight.contains("refs/heads/${default_branch}:refs/remotes/origin/${default_branch}"),
        "release preflight must fetch the live default branch into one explicit remote-tracking ref"
    );
    assert!(
        preflight.contains("refs/remotes/origin/${default_branch}"),
        "release source SHA must be compared with the live default branch tip"
    );
    assert!(
        preflight.contains("encoded_default_branch="),
        "release preflight must URI-encode the live branch name before using it as a REST path parameter"
    );
    assert!(
        preflight.contains("branches/${encoded_default_branch}"),
        "branch protection must be checked for the same URI-encoded live default branch"
    );
    assert!(
        preflight.contains("--jq '.protected'"),
        "release preflight must read the protection state for the live default branch"
    );
    assert!(
        !preflight.contains("origin develop"),
        "release preflight must not hard-code develop as publication authority"
    );
    assert!(
        !preflight.contains("branches/develop"),
        "release preflight must not hard-code develop in branch-protection admission"
    );

    let default_branch_assignment = assignment_line(preflight, "default_branch");
    assert!(
        default_branch_assignment.contains("gh api")
            && default_branch_assignment.contains("repos/${GITHUB_REPOSITORY}")
            && default_branch_assignment.contains(".default_branch"),
        "default_branch must be assigned directly from live repository metadata rather than a constant or unrelated command"
    );
    assert!(
        preflight.contains("test -n \"$default_branch\"")
            || preflight.contains("test -n \"${default_branch}\""),
        "release preflight must fail closed when live default-branch metadata is empty"
    );
    assert!(
        preflight.contains("git check-ref-format --branch \"$default_branch\"")
            || preflight.contains("git check-ref-format --branch \"${default_branch}\""),
        "release preflight must reject an invalid default-branch ref before constructing fetch authority"
    );

    let encoded_default_branch_assignment = assignment_line(preflight, "encoded_default_branch");
    assert!(
        encoded_default_branch_assignment.contains("$default_branch")
            || encoded_default_branch_assignment.contains("${default_branch}"),
        "encoded_default_branch must be derived from the exact live default_branch value"
    );
    assert!(
        encoded_default_branch_assignment.contains("@uri")
            || encoded_default_branch_assignment.contains("urllib.parse.quote"),
        "encoded_default_branch must use an actual URI encoder rather than a second branch constant"
    );

    assert_eq!(
        preflight
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("default_branch="))
            .count(),
        1,
        "release preflight must have exactly one live default_branch assignment authority"
    );
    assert_eq!(
        preflight
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("encoded_default_branch="))
            .count(),
        1,
        "release preflight must have exactly one URI-encoded branch-path authority"
    );
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
