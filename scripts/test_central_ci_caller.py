"""Contract for the product-owned immutable central CI caller."""

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]


class CentralCiCallerTests(unittest.TestCase):
    """Keep execution out of the product caller without granting write access."""

    def test_ci_is_one_immutable_central_call_without_native_execution(self):
        """A caller must pin the central implementation and carry no job steps."""
        text = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        calls = re.findall(
            r"^    uses: ContextualWisdomLab/\.github/\.github/workflows/"
            r"quarantine-sandbox-runtime-ci\.yml@([0-9a-f]{40})\s*$",
            text,
            re.MULTILINE,
        )
        self.assertEqual(len(calls), 1, "CI must call exactly one immutable central workflow")
        self.assertNotEqual(calls[0], "0" * 40, "a placeholder pin is not published authority")
        for forbidden in ("runs-on:", "steps:", "secrets: inherit", "pull_request_target:"):
            self.assertNotIn(forbidden, text)
        self.assertIn("contents: read", text)
        self.assertIn("branches: [develop]", text)
        self.assertIn("cancel-in-progress: true", text)
        self.assertIn("DISABLED/NOT_RUN", text)
        self.assertIn("podman-e2e-negative-rootless-apparmor", text)


if __name__ == "__main__":
    unittest.main()
