//! RED regression for host-side confidentiality of staged PR source.
//!
//! Container mount controls (`ro,noexec,nosuid,nodev`) constrain the sandbox,
//! but the runtime-owned temporary copy is also a host security boundary. A
//! private source tree must not become traversable/readable by unrelated local
//! users merely so a remapped container UID can consume it.

#![cfg(unix)]

use std::{fs, os::unix::fs::MetadataExt};

use quarantine_sandbox_runtime::{PrSourceArtifactInput, stage_pr_source_artifact};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn tree_digest(entries: &[(&str, &[u8])]) -> String {
    let mut hasher = Sha256::new();
    for (relative_path, contents) in entries {
        hasher.update((relative_path.len() as u64).to_be_bytes());
        hasher.update(relative_path.as_bytes());
        hasher.update((contents.len() as u64).to_be_bytes());
        hasher.update(contents);
    }
    format!("{:x}", hasher.finalize())
}

fn single_file_tree_digest(relative_path: &str, contents: &[u8]) -> String {
    tree_digest(&[(relative_path, contents)])
}

#[test]
fn nested_staged_source_preserves_payload_relative_parents() {
    let source = tempdir().expect("create source fixture");
    let entries = [
        ("nested/level/data.txt", b"nested source\n".as_slice()),
        ("nested/top.txt", b"top source\n".as_slice()),
    ];
    for (relative_path, contents) in entries {
        let path = source.path().join(relative_path);
        fs::create_dir_all(path.parent().expect("nested parent")).expect("create nested parent");
        fs::write(path, contents).expect("write nested source fixture");
    }

    let input = PrSourceArtifactInput {
        host_path: source.path().to_path_buf(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: tree_digest(&entries),
    };

    let staged = stage_pr_source_artifact(&input).expect("nested exact source should stage");
    for (relative_path, contents) in entries {
        assert_eq!(
            fs::read(staged.mount_path().join(relative_path)).expect("read nested staged bytes"),
            contents
        );
    }
    assert_eq!(
        fs::metadata(staged.mount_path().join("nested"))
            .expect("nested staged directory metadata")
            .mode()
            & 0o777,
        0o755
    );
}

#[test]
fn staged_source_root_is_not_accessible_to_unrelated_host_users() {
    let source = tempdir().expect("create source fixture");
    let relative_path = "private-source.txt";
    let contents = b"private repository source\n";
    fs::write(source.path().join(relative_path), contents).expect("write source fixture");

    let input = PrSourceArtifactInput {
        host_path: source.path().to_path_buf(),
        revision_sha: "a".repeat(40),
        expected_tree_sha256: single_file_tree_digest(relative_path, contents),
    };

    let staged = stage_pr_source_artifact(&input).expect("valid exact source should stage");
    let staged_path = staged.private_path().to_path_buf();
    let mount_path = staged.mount_path();
    assert_eq!(
        staged.path(),
        mount_path,
        "adapter-facing path must select the inner payload"
    );
    assert_eq!(
        mount_path.parent(),
        Some(staged_path.as_path()),
        "admitted bind source must be a child of the private root"
    );
    assert_ne!(
        mount_path,
        staged.private_path(),
        "private root must not itself be bound"
    );
    let payload_mode = fs::metadata(mount_path)
        .expect("mountable payload metadata")
        .mode()
        & 0o777;
    assert_eq!(
        payload_mode, 0o755,
        "auto-remapped nonroot user needs readable mount root"
    );
    assert_eq!(
        fs::read(mount_path.join(relative_path)).expect("owner can read staged bytes"),
        contents
    );
    let root_mode = fs::metadata(&staged_path)
        .expect("staging root metadata")
        .mode()
        & 0o777;

    assert_eq!(
        root_mode & 0o077,
        0,
        "runtime-owned staging root must not grant group/other host traversal; got mode {root_mode:o}"
    );
    assert_eq!(staged.receipt().tree_sha256(), input.expected_tree_sha256);
    assert_eq!(staged.receipt().regular_file_count(), 1);
    assert_eq!(staged.receipt().total_bytes(), contents.len() as u64);
    assert!(staged.receipt().mounted_read_only());
    assert!(staged.receipt().mounted_noexec());

    drop(staged);
    assert!(
        !staged_path.exists(),
        "runtime-owned source staging must still be deleted at end of lifetime"
    );
}
