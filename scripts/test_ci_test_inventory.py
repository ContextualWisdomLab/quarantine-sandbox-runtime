"""Keep a failing Rust target from hiding later security regressions in native CI."""

from pathlib import Path
import re
import shlex
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/ci.yml"
EXPECTED_TEST_ARGV = [
    "cargo", "test", "--locked", "--workspace", "--all-targets", "--no-fail-fast"
]


class NativeTestInventoryContract(unittest.TestCase):
    """Pin the native evidence path without granting a failing job success."""

    def setUp(self) -> None:
        """Read the checked-in workflow rather than a second fixture copy."""
        self.workflow = WORKFLOW.read_text(encoding="utf-8")
        jobs = re.findall(
            r"(?ms)^  verify:\n(.*?)(?=^  [a-zA-Z_][\w-]*:\n|\Z)",
            self.workflow,
        )
        self.assertEqual(len(jobs), 1, "one canonical native verification job is required")
        self.verify_job = jobs[0]

    def test_all_rust_targets_execute_without_a_test_name_filter(self) -> None:
        """Preserve Cargo-level target enumeration and every original failing status."""
        steps = re.findall(
            r"(?ms)^      - name: Test\n(.*?)(?=^      - |\Z)", self.verify_job
        )
        self.assertEqual(len(steps), 1, "one native Rust Test step is required")
        fields = [line.strip() for line in steps[0].splitlines() if line.strip()]
        self.assertEqual(len(fields), 1, "the Test step must not mask or condition failures")
        self.assertTrue(fields[0].startswith("run: "))
        self.assertEqual(
            shlex.split(fields[0].removeprefix("run: ")), EXPECTED_TEST_ARGV,
            "run every Cargo target; do not filter tests or swallow the final failure",
        )

    def test_native_job_cannot_promote_test_failure_to_success(self) -> None:
        """Reject job- or step-level error suppression, including expression forms."""
        suppression = re.findall(
            r"(?m)^\s*continue-on-error:\s*(\S.*?)\s*$", self.verify_job
        )
        self.assertTrue(
            all(value == "false" for value in suppression),
            "continue-on-error must not turn a security regression into a green job",
        )

    def test_inventory_contract_runs_before_rust_test_execution(self) -> None:
        """The contract must be executed by CI, not merely checked into the repository."""
        command = (
            "run: python3 -m unittest scripts/test_check_coverage.py "
            "scripts/test_ci_test_inventory.py"
        )
        self.assertIn(command, self.verify_job)
        self.assertLess(self.verify_job.index(command), self.verify_job.index("- name: Test\n"))


if __name__ == "__main__":
    unittest.main()
