//! Regression witness for LLVM half-open source-line diagnostics.
//!
//! The coverage admission path already treats a multi-line region ending at
//! column 1 as half-open. Diagnostic output must use the same source interval
//! or it can direct a repair toward a line that is not part of the region.

use std::process::Command;

#[test]
fn uncovered_line_diagnostics_follow_half_open_llvm_regions() {
    let python = r#"
from scripts.check_coverage import _uncovered_lines

filename = "/workspace/src/runtime.rs"

def uncovered(region):
    data = {
        "functions": [{
            "filenames": [filename],
            "regions": [region],
        }]
    }
    return _uncovered_lines(data, filename)

print(uncovered([10, 5, 11, 1, 0, 0, 0, 0]))
print(uncovered([10, 5, 11, 2, 0, 0, 0, 0]))
print(uncovered([10, 5, 10, 5, 0, 0, 0, 0]))
"#;

    let output = Command::new("python3")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["-c", python])
        .output()
        .expect("repository coverage policy tests require python3");

    assert!(
        output.status.success(),
        "coverage diagnostic probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "[10]\n[10, 11]\n[10]\n",
        "uncovered-line diagnostics must share the coverage admission path's half-open terminal-line rule"
    );
}
