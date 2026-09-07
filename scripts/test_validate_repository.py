#!/usr/bin/env python3
"""Regression tests for repository workflow action pin validation."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import pathlib
import tempfile
import unittest

VALIDATOR_PATH = pathlib.Path(__file__).with_name("validate_repository.py")
DRAFT_2020_12 = "https://json-schema.org/draft/2020-12/schema"
PINNED_SHA = "a" * 40


def load_validator():
    """Load a fresh validator module so each test can replace its repository root."""
    spec = importlib.util.spec_from_file_location("qsr_validate_repository", VALIDATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("repository validator module must be loadable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_required_repository(root: pathlib.Path, validator) -> None:
    """Create the smallest repository tree that satisfies non-workflow policy checks."""
    schema_files = set(validator.SCHEMA_FILES)
    for relative_path in validator.REQUIRED_FILES:
        path = root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        if relative_path in schema_files:
            path.write_text(
                json.dumps(
                    {
                        "$schema": DRAFT_2020_12,
                        "additionalProperties": False,
                    }
                ),
                encoding="utf-8",
            )
        else:
            path.write_text("", encoding="utf-8")


def write_workflow(root: pathlib.Path, name: str, action_reference: str) -> None:
    """Write one minimal workflow containing a single action reference."""
    workflow_path = root / ".github" / "workflows" / name
    workflow_path.parent.mkdir(parents=True, exist_ok=True)
    workflow_path.write_text(
        "\n".join(
            [
                "name: workflow-pin-policy-fixture",
                "on: push",
                "jobs:",
                "  verify:",
                "    runs-on: ubuntu-24.04",
                "    steps:",
                f"      - uses: actions/checkout@{action_reference}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def run_validator(root: pathlib.Path) -> tuple[int, str]:
    """Run repository validation against an isolated fixture root."""
    validator = load_validator()
    validator.ROOT = root
    stderr = io.StringIO()
    with contextlib.redirect_stderr(stderr):
        result = validator.main()
    return result, stderr.getvalue()


class WorkflowActionPinPolicyTests(unittest.TestCase):
    """Require immutable action pins across every workflow file."""

    def new_repository(self) -> tuple[tempfile.TemporaryDirectory[str], pathlib.Path]:
        """Return a temporary repository with all unrelated policy prerequisites present."""
        temporary_directory = tempfile.TemporaryDirectory()
        root = pathlib.Path(temporary_directory.name)
        validator = load_validator()
        write_required_repository(root, validator)
        return temporary_directory, root

    def test_unpinned_action_in_second_yml_workflow_fails(self) -> None:
        temporary_directory, root = self.new_repository()
        with temporary_directory:
            write_workflow(root, "ci.yml", PINNED_SHA)
            write_workflow(root, "secondary.yml", "v1")

            result, stderr = run_validator(root)

            self.assertEqual(result, 1)
            self.assertIn("workflow action is not pinned by commit SHA", stderr)

    def test_unpinned_action_in_yaml_workflow_fails(self) -> None:
        temporary_directory, root = self.new_repository()
        with temporary_directory:
            write_workflow(root, "ci.yml", PINNED_SHA)
            write_workflow(root, "security.yaml", "v2")

            result, stderr = run_validator(root)

            self.assertEqual(result, 1)
            self.assertIn("workflow action is not pinned by commit SHA", stderr)

    def test_missing_workflow_directory_is_a_policy_failure(self) -> None:
        temporary_directory, root = self.new_repository()
        with temporary_directory:
            try:
                result, stderr = run_validator(root)
            except OSError as exc:
                self.fail(f"missing workflows must return a policy failure, not raise {exc!r}")

            self.assertEqual(result, 1)
            self.assertIn("workflow", stderr.lower())

    def test_multiple_sha_pinned_workflows_remain_valid(self) -> None:
        temporary_directory, root = self.new_repository()
        with temporary_directory:
            write_workflow(root, "ci.yml", PINNED_SHA)
            write_workflow(root, "security.yaml", "b" * 40)

            result, stderr = run_validator(root)

            self.assertEqual(result, 0, stderr)


if __name__ == "__main__":
    unittest.main()
