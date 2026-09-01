//! Architectural fitness tests for DDD ownership and documentation identity.

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
        assert_eq!(number.len(), 4, "ADR identifier must be four digits: {name}");
        assert!(
            number.bytes().all(|byte| byte.is_ascii_digit()),
            "ADR identifier must be numeric: {name}"
        );
        assert!(numbers.insert(number.to_owned()), "duplicate ADR {number}");
    }
}
