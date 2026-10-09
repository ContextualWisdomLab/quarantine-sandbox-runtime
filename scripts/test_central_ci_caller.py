"""Contract for the product-owned immutable central CI caller.

This stdlib-only oracle recognizes the published canonical YAML form, not general
YAML: blank and whole-line comments outside folded scalars are ignored, but
aliases, anchors, flow mappings, alternate indentation, inline comments,
duplicate keys, non-scalar mapping values, and reordered/additional fields are
rejected. Unsupported syntax must be reviewed before changing the caller form.
"""

import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

# All meaningful canonical lines are required in order. This bounds YAML
# interpretation without introducing PyYAML as a product/test dependency.
CANONICAL_LINES = (
    "name: CI",
    "on:",
    "  pull_request:",
    "  push:",
    "    branches: [develop]",
    "permissions:",
    "  contents: read",
    "concurrency:",
    "  group: >-",
    "    qsr-caller-${{ github.workflow }}-${{ github.repository }}-${{",
    "      github.event_name == 'pull_request' && github.event.pull_request.number || github.run_id",
    "    }}",
    "  cancel-in-progress: true",
    "jobs:",
    "  ci:",
    "    uses: ContextualWisdomLab/.github/.github/workflows/"
    "quarantine-sandbox-runtime-ci.yml@91f0ccc23fa19b82ce797573155bb6f6a0f64b5f",
)


class CentralCiCallerTests(unittest.TestCase):
    """Keep execution out of the product caller without granting write access."""

    def test_ci_is_one_immutable_central_call_without_native_execution(self):
        """The sole job is the published producer, with no override surface."""
        text = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        raw_lines = text.splitlines()
        self.assertIn("  group: >-", raw_lines, "unsupported concurrency scalar form")
        start = raw_lines.index("  group: >-")
        self.assertEqual(
            tuple(raw_lines[start:start + 4]), CANONICAL_LINES[8:12],
            "folded concurrency scalar must retain its exact physical lines",
        )
        self.assertEqual(
            raw_lines[start + 4:start + 5], ["  cancel-in-progress: true"],
            "unexpected folded scalar continuation or missing cancellation",
        )
        lines = tuple(
            line for line in raw_lines
            if line.strip() and not line.lstrip().startswith("#")
        )
        self.assertEqual(lines, CANONICAL_LINES, "CI caller differs from canonical published profile")
        self.assertIn("DISABLED/NOT_RUN", text)
        self.assertIn("podman-e2e-negative-rootless-apparmor", text)


if __name__ == "__main__":
    unittest.main()
