//! Repository-level release delivery contract.
//!
//! These tests intentionally inspect checked-in release automation rather than
//! executing publication. They keep release evidence fail-closed and reviewable.

use std::{fs, path::Path};

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
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
