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
        let production_source = strip_trailing_test_module(&source);
        for forbidden in [".unwrap(", ".expect(", "panic!("] {
            assert!(
                !production_source.contains(forbidden),
                "production source {} contains forbidden panic shortcut {forbidden}",
                path.display()
            );
        }
    }
}

/// Drop a trailing `#[cfg(test)]`/`#[cfg(all(test, ...))] mod tests { ... }`
/// block before scanning for forbidden panic shortcuts.
///
/// This crate's convention (enforced by every `src/**/*.rs` file that has
/// one) is a single test module as the file's last item, gated by a `#[cfg]`
/// attribute naming `test` on the line directly above `mod tests {`. That
/// module never ships in a production build, so a panic shortcut inside it
/// is normal Rust test style, not a production-safety violation; only the
/// text before it is production source.
fn strip_trailing_test_module(source: &str) -> &str {
    let lines: Vec<&str> = source.lines().collect();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let test_only_cfg = trimmed == "#[cfg(test)]"
            || (trimmed.starts_with("#[cfg(all(test,") && trimmed.ends_with("))]"));
        if test_only_cfg {
            let starts_test_module = lines
                .get(index + 1)
                .is_some_and(|next| next.trim_start().starts_with("mod tests"));
            if starts_test_module {
                let prefix_len: usize = lines[..index].iter().map(|line| line.len() + 1).sum();
                let module_start = prefix_len
                    + line.len()
                    + 1
                    + lines[index + 1]
                        .find('{')
                        .expect("test module declaration must open a block");
                let mut depth = 0_i32;
                for (offset, byte) in source[module_start..].bytes().enumerate() {
                    match byte {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                let after_module = module_start + offset + 1;
                                if source[after_module..].trim().is_empty() {
                                    return &source[..prefix_len.min(source.len())];
                                }
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    source
}

#[test]
fn test_module_stripping_never_hides_conditionally_production_or_trailing_items() {
    let conditional = "#[cfg(any(test, feature = \"unsafe-shortcut\"))]\nmod tests { fn helper() { panic!(); } }\n";
    assert_eq!(strip_trailing_test_module(conditional), conditional);

    let trailing =
        "#[cfg(test)]\nmod tests { fn helper() { panic!(); } }\nfn production() { panic!(); }\n";
    assert_eq!(strip_trailing_test_module(trailing), trailing);

    let test_only = "fn production() {}\n#[cfg(test)]\nmod tests { fn helper() { panic!(); } }\n";
    assert_eq!(
        strip_trailing_test_module(test_only),
        "fn production() {}\n"
    );
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
