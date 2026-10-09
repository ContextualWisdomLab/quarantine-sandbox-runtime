"""Synthetic mutation tests for the exact immutable QSR central CI caller profile.

Run with ``python3 -B -m unittest scripts/test_central_ci_caller_sensitivity.py``.
The oracle deliberately accepts only the published canonical-form workflow;
unsupported YAML constructs are rejected, not interpreted or normalized.
"""

import importlib.util
import pathlib
import tempfile
import unittest
from unittest.mock import patch


SCRIPTS = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("qsr_caller_oracle", SCRIPTS / "test_central_ci_caller.py")
caller = importlib.util.module_from_spec(spec)
spec.loader.exec_module(caller)
CANDIDATE = (SCRIPTS.parent / ".github/workflows/ci.yml").read_text(encoding="utf-8")


class CallerMutationTests(unittest.TestCase):
    def run_candidate(self, workflow):
        with tempfile.TemporaryDirectory(prefix="qsr-caller-sensitivity-") as directory:
            root = pathlib.Path(directory)
            path = root / ".github/workflows/ci.yml"
            path.parent.mkdir(parents=True)
            path.write_text(workflow, encoding="utf-8")
            with patch.object(caller, "ROOT", root):
                result = unittest.TestResult()
                caller.CentralCiCallerTests(
                    "test_ci_is_one_immutable_central_call_without_native_execution"
                ).run(result)
            self.assertEqual(result.testsRun, 1)
            self.assertFalse(result.errors, result.errors)
            return result

    def test_actual_published_candidate_is_accepted(self):
        result = self.run_candidate(CANDIDATE)
        self.assertTrue(result.wasSuccessful(), result.failures)

    def test_actions_write_is_assertion_failure(self):
        mutated = CANDIDATE.replace("  contents: read", "  contents: read\n  actions: write")
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_collision_prefix_is_assertion_failure(self):
        result = self.run_candidate(CANDIDATE.replace("qsr-caller-", "qsr-central-"))
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_other_full_sha_is_assertion_failure(self):
        mutated = CANDIDATE.replace("91f0ccc23fa19b82ce797573155bb6f6a0f64b5f", "1" * 40)
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_duplicate_jobs_mapping_is_assertion_failure(self):
        result = self.run_candidate(CANDIDATE + "\njobs:\n  ci:\n    uses: another/reusable.yml@main\n")
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_nonscalar_permissions_mapping_is_assertion_failure(self):
        mutated = CANDIDATE.replace("  contents: read", "  contents:\n    nested: read")
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_input_override_is_assertion_failure(self):
        result = self.run_candidate(CANDIDATE + "\n    with:\n      skip: true\n")
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_secret_override_is_assertion_failure(self):
        result = self.run_candidate(CANDIDATE + "\n    secrets:\n      TOKEN: '${{ secrets.TOKEN }}'\n")
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_trigger_removed_is_assertion_failure(self):
        result = self.run_candidate(CANDIDATE.replace("  pull_request:\n", ""))
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_blank_inside_folded_group_is_assertion_failure(self):
        mutated = CANDIDATE.replace("      github.event_name", "\n      github.event_name")
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_comment_after_folded_group_is_assertion_failure(self):
        mutated = CANDIDATE.replace("    }}\n", "    }}\n    # scalar content, not a YAML comment\n")
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_truncated_after_folded_group_is_assertion_failure(self):
        mutated = CANDIDATE.split("  cancel-in-progress: true")[0]
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)

    def test_canonical_profile_mutation_matrix_is_assertion_failures(self):
        """Characterize already-enforced syntax/contract edges, one result per case."""
        cases = {
            "protected_push_removed": CANDIDATE.replace("  push:\n    branches: [develop]\n", ""),
            "pull_request_filter_added": CANDIDATE.replace("  pull_request:\n", "  pull_request:\n    branches: [develop]\n"),
            "develop_branch_changed": CANDIDATE.replace("branches: [develop]", "branches: [main]"),
            "cancel_false": CANDIDATE.replace("cancel-in-progress: true", "cancel-in-progress: false"),
            "repository_scope_removed": CANDIDATE.replace("-${{ github.repository }}", ""),
            "pr_number_replaced": CANDIDATE.replace("github.event.pull_request.number", "github.ref"),
            "push_run_id_replaced": CANDIDATE.replace("github.run_id", "github.ref"),
            "duplicate_permissions": CANDIDATE + "\npermissions:\n  actions: write\n",
            "duplicate_contents": CANDIDATE.replace("  contents: read", "  contents: read\n  contents: write"),
            "duplicate_ci": CANDIDATE + "\n  ci:\n    uses: another/reusable.yml@main\n",
            "duplicate_uses": CANDIDATE + "\n    uses: another/reusable.yml@main\n",
            "mapping_job_name": CANDIDATE.replace("  ci:", "  [ci]:"),
            "mapping_contents": CANDIDATE.replace("  contents: read", "  contents: {level: read}"),
            "job_env": CANDIDATE + "\n    env:\n      OVERRIDE: true\n",
            "job_if": CANDIDATE + "\n    if: false\n",
            "job_permissions": CANDIDATE + "\n    permissions:\n      actions: write\n",
            "secret_inherit": CANDIDATE + "\n    secrets: inherit\n",
            "native_steps": CANDIDATE + "\n    steps: []\n",
            "native_runner": CANDIDATE + "\n    runs-on: ubuntu-latest\n",
            "short_sha": CANDIDATE.replace("91f0ccc23fa19b82ce797573155bb6f6a0f64b5f", "91f0ccc"),
            "anchor": CANDIDATE.replace("permissions:", "permissions: &permissions"),
            "alias": CANDIDATE.replace("  contents: read", "  contents: *permission"),
            "flow_jobs": CANDIDATE.replace("jobs:\n  ci:", "jobs: {ci: {}}\n  ci:"),
            "inline_comment": CANDIDATE.replace("  contents: read", "  contents: read # unsupported"),
            "alternate_indent": CANDIDATE.replace("  ci:", " ci:"),
            "duplicate_on": CANDIDATE + "\non:\n  workflow_dispatch:\n",
            "nonscalar_uses": CANDIDATE.replace("    uses: ", "    uses:\n      nested: "),
            "folded_comment_middle": CANDIDATE.replace("      github.event_name", "    # scalar content\n      github.event_name"),
        }
        for label, mutated in cases.items():
            with self.subTest(case=label):
                self.assertNotEqual(mutated, CANDIDATE)
                result = self.run_candidate(mutated)
                self.assertEqual(len(result.failures), 1, result.failures)

    def test_extra_job_is_assertion_failure(self):
        mutated = CANDIDATE + "\n  unexpected:\n    uses: another/reusable/workflow.yml@main\n"
        result = self.run_candidate(mutated)
        self.assertEqual(len(result.failures), 1, result.failures)


if __name__ == "__main__":
    unittest.main()
