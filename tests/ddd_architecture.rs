//! Architectural fitness tests for DDD ownership and production safety rules.

use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn podman_adapter_lives_outside_core_sandbox_context() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    assert!(root.join("src/infrastructure/podman.rs").is_file());
    assert!(!root.join("src/sandbox_execution/podman.rs").exists());

    let core = fs::read_to_string(root.join("src/sandbox_execution/mod.rs"))
        .expect("sandbox_execution source should be readable");
    assert!(!core.contains("application_service"));
    assert!(!core.contains("ApplicationService"));
}

#[test]
fn accepted_adr_numbers_are_unique() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/adr");
    let mut numbers = BTreeSet::new();

    for entry in fs::read_dir(root).expect("ADR directory should be readable") {
        let entry = entry.expect("ADR entry should be readable");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "README.md" || !name.ends_with(".md") {
            continue;
        }
        let number = name
            .split_once('-')
            .map(|(number, _)| number)
            .expect("ADR filename should have a numeric prefix and dash");
        assert_eq!(
            number.len(),
            4,
            "ADR identifier must be four digits: {name}"
        );
        assert!(
            number.bytes().all(|byte| byte.is_ascii_digit()),
            "ADR identifier must be numeric: {name}"
        );
        assert!(numbers.insert(number.to_owned()), "duplicate ADR {number}");
    }
}

#[test]
fn production_source_has_no_panic_shortcuts() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source_files = Vec::new();
    collect_rust_sources(&source_root, &mut source_files);

    assert!(
        !source_files.is_empty(),
        "production Rust sources must exist"
    );
    for path in source_files {
        let source = fs::read_to_string(&path).expect("production source should be readable");
        for forbidden in [".unwrap(", ".expect(", "panic!("] {
            assert!(
                !source.contains(forbidden),
                "production source {} contains forbidden panic shortcut {forbidden}",
                path.display()
            );
        }
    }
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory should be readable") {
        let entry = entry.expect("source entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
