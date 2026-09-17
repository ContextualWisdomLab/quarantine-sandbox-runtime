//! Regression witness for branch coverage across Rust codegen instantiations.
//!
//! LLVM can export the same physical source branch once per monomorphized
//! function. Coverage admission must union each branch outcome by source
//! identity, just as the repository already does for source lines and regions,
//! or complementary executions can be reported as an artificial uncovered
//! branch.

use std::process::Command;

#[test]
fn branch_admission_unions_complementary_codegen_instances() {
    let python = r#"
import io
import json
import pathlib
import tempfile
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

from scripts.check_coverage import main

filename = "/workspace/src/runtime.rs"


def payload(second_false_count):
    return {
        "data": [{
            "totals": {
                "lines": {"count": 1, "covered": 1},
                "functions": {"count": 2, "covered": 2},
                "regions": {"count": 1, "covered": 1},
                "branches": {"count": 2, "covered": 1},
            },
            "files": [{
                "filename": filename,
                "summary": {
                    "lines": {"count": 1, "covered": 1},
                    "functions": {"count": 2, "covered": 2},
                    "regions": {"count": 1, "covered": 1},
                    "branches": {"count": 2, "covered": 1},
                },
                "segments": [
                    [10, 5, 1, True, True, False],
                    [10, 20, 0, False, False, False],
                ],
            }],
            "functions": [
                {
                    "filenames": [filename],
                    "regions": [[10, 5, 10, 20, 1, 0, 0, 0]],
                    "branches": [[10, 8, 10, 18, 1, 0, 0, 0, 4]],
                },
                {
                    "filenames": [filename],
                    "regions": [[10, 5, 10, 20, 1, 0, 0, 0]],
                    "branches": [[10, 8, 10, 18, 0, second_false_count, 0, 0, 4]],
                },
            ],
        }]
    }


def run(candidate):
    with tempfile.TemporaryDirectory() as directory:
        path = pathlib.Path(directory) / "branch-coverage.json"
        path.write_text(json.dumps(candidate), encoding="utf-8")
        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            patch("sys.argv", ["check_coverage.py", "--require-branches", str(path)]),
            redirect_stdout(stdout),
            redirect_stderr(stderr),
        ):
            result = main()
    return result, stdout.getvalue(), stderr.getvalue()

covered_result, covered_stdout, covered_stderr = run(payload(1))
assert covered_result == 0, (covered_stdout, covered_stderr)
assert covered_stderr == "", covered_stderr
assert "branches (LLVM raw): 1/2" in covered_stdout, covered_stdout
assert "branches: 2/2" in covered_stdout, covered_stdout
assert "incomplete file" not in covered_stdout, covered_stdout

uncovered_result, uncovered_stdout, uncovered_stderr = run(payload(0))
assert uncovered_result == 1, (uncovered_stdout, uncovered_stderr)
assert "branches coverage is 1/2" in uncovered_stderr, uncovered_stderr
"#;

    let output = Command::new("python3")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["-c", python])
        .output()
        .expect("repository coverage policy tests require python3");

    assert!(
        output.status.success(),
        "branch codegen-union probe failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
