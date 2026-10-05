"""Offline limited-observation receipt checks; fixtures are synthetic."""

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.audit_runner_boundary_receipt import audit_receipt


class LimitedReceiptAuditTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.serials = {}
        self.rows = {}
        transaction = "a" * 32
        for role in ("control", "denied"):
            guest = {
                "transaction": transaction,
                "hostname": "sdp-vm-" + role,
                "pristine_paths": True,
                "host_gateway": "connected" if role == "control" else "TimeoutError",
                "host_lan": "connected" if role == "control" else "TimeoutError",
                "external": "connected",
            }
            serial = (
                b"synthetic boot log\nSDP_VM_RESULT="
                + json.dumps(guest).encode()
                + b"\n"
            )
            path = self.root / (role + ".log")
            path.write_bytes(serial)
            self.serials[role] = path
            unit = "sdp-vm-boundary-" + role + "-20261003.service"
            after = {
                "MainPID": "0",
                "ControlGroup": "",
                "IPAddressDeny": "",
                "User": "",
                "Description": unit,
                "LoadState": "not-found",
                "ActiveState": "inactive",
                "InvocationID": "",
            }
            cgroup = "/system.slice/" + unit
            self.rows[role] = dict(
                guest,
                attachments={} if role == "control" else {"0": [5606], "1": [5605]},
                qemu_identity={
                    "pid": 101 if role == "control" else 102,
                    "invocation": ("b" if role == "control" else "c") * 32,
                    "cgroup": cgroup,
                    "process": {
                        "uid": 995,
                        "cgroup": "0::" + cgroup,
                        "exe": "/usr/bin/qemu-system-x86_64",
                        "start": "100",
                    },
                },
                fixture_hits=["host_gateway", "host_lan"] if role == "control" else [],
                wrapper_exit=0,
                unit_after_exit=after,
                serial_sha256=hashlib.sha256(serial).hexdigest(),
            )
        self.summary = {
            "passed_limited_vm_boundary": True,
            "results": self.rows,
            "settlement": {
                "units": {
                    "False": self.rows["control"]["unit_after_exit"],
                    "True": self.rows["denied"]["unit_after_exit"],
                },
                "wrappers": {"False": 0, "True": 0},
                "errors": [],
                "passed": True,
                "owned_pids": [],
                "existing_pids_preserved": True,
                "receipt_fsynced": True,
            },
            "whole_cidr_proof": False,
            "full_clean_job": False,
            "registered_ci_accepted": False,
        }
        self.path = self.root / "summary.json"
        self.save()

    def save(self):
        self.path.write_text(json.dumps(self.summary), encoding="utf-8")
        self.digest = hashlib.sha256(self.path.read_bytes()).hexdigest()

    def audit(self):
        return audit_receipt(
            self.path, self.serials["control"], self.serials["denied"], self.digest
        )

    def test_valid_bound_observation_is_never_lease_authority(self):
        result = self.audit()
        self.assertTrue(result["observation_consistent"])
        self.assertFalse(result["lease_allowed"])
        self.assertFalse(result["operator_authenticated"])
        self.assertEqual(result["scope"], "limited_vm_point_observation")

    def test_rejects_identity_types_and_unjoined_process_or_cleanup(self):
        mutations = [
            (self.rows["denied"]["qemu_identity"], "pid", True),
            (self.rows["denied"]["qemu_identity"]["process"], "uid", True),
            (self.rows["denied"]["qemu_identity"]["process"], "cgroup", "0::/foreign"),
            (self.rows["denied"]["qemu_identity"], "invocation", "not-an-invocation"),
            (self.rows["denied"], "attachments", {"0": [True], "1": [1]}),
            (self.rows["denied"], "attachments", {"0": [], "1": [1]}),
            (self.rows["denied"], "wrapper_exit", True),
            (self.rows["denied"]["unit_after_exit"], "MainPID", "22"),
            (self.rows["control"], "fixture_hits", []),
        ]
        for section, key, value in mutations:
            with self.subTest(key=key, value=value):
                previous = section[key]
                try:
                    section[key] = value
                    self.save()
                    with self.assertRaises(ValueError):
                        self.audit()
                finally:
                    section[key] = previous

    def test_rejects_unbound_transaction_and_missing_guest_fields(self):
        self.rows["denied"]["transaction"] = "d" * 32
        serial = (
            b"SDP_VM_RESULT="
            + json.dumps(
                {
                    key: self.rows["denied"][key]
                    for key in (
                        "transaction",
                        "hostname",
                        "pristine_paths",
                        "host_gateway",
                        "host_lan",
                        "external",
                    )
                }
            ).encode()
            + b"\n"
        )
        self.serials["denied"].write_bytes(serial)
        self.rows["denied"]["serial_sha256"] = hashlib.sha256(serial).hexdigest()
        self.save()
        with self.assertRaises(ValueError):
            self.audit()

    def test_rejects_duplicate_keys_oversize_and_incomplete_guest(self):
        for raw in (b'{"results":{},"results":{}}', b" " * (262144 + 1)):
            with self.subTest(size=len(raw)):
                self.path.write_bytes(raw)
                digest = hashlib.sha256(raw).hexdigest()
                with self.assertRaises(ValueError):
                    audit_receipt(
                        self.path,
                        self.serials["control"],
                        self.serials["denied"],
                        digest,
                    )
        self.save()
        raw = b"SDP_VM_RESULT={}\n"
        self.serials["control"].write_bytes(raw)
        self.rows["control"]["serial_sha256"] = hashlib.sha256(raw).hexdigest()
        self.save()
        with self.assertRaises(ValueError):
            self.audit()

    def test_cli_returns_non_authoritative_json_and_sanitized_rejection(self):
        script = Path(__file__).with_name("audit_runner_boundary_receipt.py")
        args = [
            sys.executable,
            str(script),
            "--summary",
            str(self.path),
            "--control-serial",
            str(self.serials["control"]),
            "--denied-serial",
            str(self.serials["denied"]),
            "--expected-summary-sha256",
            self.digest,
        ]
        result = subprocess.run(
            args, capture_output=True, text=True, timeout=10, check=False
        )
        self.assertEqual(result.returncode, 0)
        self.assertFalse(json.loads(result.stdout)["lease_allowed"])
        self.path.write_bytes(b"private diagnostic contents")
        result = subprocess.run(
            args, capture_output=True, text=True, timeout=10, check=False
        )
        self.assertEqual(result.returncode, 1)
        output = json.loads(result.stdout)
        self.assertFalse(output["observation_consistent"])
        self.assertFalse(output["lease_allowed"])
        self.assertNotIn("private diagnostic", result.stdout + result.stderr)

    def test_malformed_json_types_and_serial_binding_fail_closed(self):
        original = self.path.read_bytes()
        for raw in (
            b"null",
            b"[]",
            b'{"x": NaN}',
            b'{"x": 1e999}',
            b'{"x": "\\ud800"}',
            b"[" * 18 + b"0" + b"]" * 18,
            b"\xff",
            b"invalid",
        ):
            with self.subTest(raw=raw[:30]):
                self.path.write_bytes(raw)
                with self.assertRaises(ValueError):
                    audit_receipt(
                        self.path,
                        self.serials["control"],
                        self.serials["denied"],
                        hashlib.sha256(raw).hexdigest(),
                    )
        self.path.write_bytes(original)
        with self.assertRaises(ValueError):
            audit_receipt(
                self.path, self.serials["control"], self.serials["denied"], "0" * 64
            )
        for raw in (
            b"no result",
            self.serials["control"].read_bytes() * 2,
            b"SDP_VM_RESULT="
            + json.dumps(dict(self.rows["control"], extra="x")).encode(),
        ):
            with self.subTest(serial=raw[:20]):
                self.serials["control"].write_bytes(raw)
                self.rows["control"]["serial_sha256"] = hashlib.sha256(raw).hexdigest()
                self.save()
                with self.assertRaises(ValueError):
                    self.audit()
        self.serials["control"].write_bytes(b"different hash")
        with self.assertRaises(ValueError):
            self.audit()

    def test_input_reader_rejects_missing_symlink_fifo_and_overlimit(self):
        for kind in ("missing", "symlink", "fifo", "oversize"):
            with self.subTest(kind=kind):
                path = self.root / kind
                if kind == "symlink":
                    path.symlink_to(self.path)
                elif kind == "fifo":
                    import os

                    os.mkfifo(path)
                elif kind == "oversize":
                    path.write_bytes(b" " * (262144 + 1))
                with self.assertRaises(ValueError):
                    audit_receipt(
                        path,
                        self.serials["control"],
                        self.serials["denied"],
                        self.digest,
                    )

    def test_additional_identity_observation_and_cleanup_mutations_reject(self):
        cases = [
            (self.rows["denied"]["qemu_identity"], "pid", 0),
            (self.rows["denied"]["qemu_identity"], "pid", 101),
            (self.rows["denied"]["qemu_identity"], "invocation", "b" * 32),
            (self.rows["denied"]["qemu_identity"]["process"], "exe", "/wrong"),
            (self.rows["denied"]["qemu_identity"]["process"], "start", "0"),
            (self.rows["denied"], "attachments", {"0": [1, 1], "1": [2]}),
            (self.rows["control"], "attachments", {"0": [1]}),
            (self.rows["denied"], "pristine_paths", 1),
            (self.rows["denied"], "host_gateway", "connected"),
            (self.summary["settlement"]["wrappers"], "True", 1),
            (self.summary["settlement"]["units"], "True", {}),
        ]
        for section, key, value in cases:
            with self.subTest(key=key):
                old = section[key]
                try:
                    section[key] = value
                    self.save()
                    with self.assertRaises(ValueError):
                        self.audit()
                finally:
                    section[key] = old

    def test_main_success_rejection_and_entrypoint(self):
        import contextlib
        import io
        import runpy
        from unittest.mock import patch

        from scripts.audit_runner_boundary_receipt import main

        args = [
            "audit",
            "--summary",
            str(self.path),
            "--control-serial",
            str(self.serials["control"]),
            "--denied-serial",
            str(self.serials["denied"]),
            "--expected-summary-sha256",
            self.digest,
        ]
        with (
            patch.object(sys, "argv", args),
            contextlib.redirect_stdout(io.StringIO()) as output,
        ):
            self.assertEqual(main(), 0)
            self.assertFalse(json.loads(output.getvalue())["lease_allowed"])
        self.path.unlink()
        with (
            patch.object(sys, "argv", args),
            contextlib.redirect_stdout(io.StringIO()) as output,
        ):
            self.assertEqual(main(), 1)
            self.assertEqual(json.loads(output.getvalue())["error"], "receipt_rejected")
        with patch.object(sys, "argv", args), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                runpy.run_path(
                    str(Path(__file__).with_name("audit_runner_boundary_receipt.py")),
                    run_name="__main__",
                )
            self.assertEqual(raised.exception.code, 1)

    def test_exact_input_limits_accept_and_one_byte_over_rejects(self):
        base = self.path.read_bytes()
        self.path.write_bytes(base + b" " * (262144 - len(base)))
        self.digest = hashlib.sha256(self.path.read_bytes()).hexdigest()
        self.assertTrue(self.audit()["observation_consistent"])
        serial = self.serials["control"].read_bytes()
        serial += b"x" * (1048576 - len(serial))
        self.serials["control"].write_bytes(serial)
        self.rows["control"]["serial_sha256"] = hashlib.sha256(serial).hexdigest()
        self.save()
        self.assertTrue(self.audit()["observation_consistent"])
        self.serials["control"].write_bytes(serial + b"x")
        self.rows["control"]["serial_sha256"] = hashlib.sha256(
            serial + b"x"
        ).hexdigest()
        self.save()
        with self.assertRaises(ValueError):
            self.audit()

    def test_rejects_false_cleanup_and_promoted_authority_flags(self):
        for section, key, value in [
            (self.summary, "whole_cidr_proof", True),
            (self.summary, "full_clean_job", True),
            (self.summary, "registered_ci_accepted", True),
            (self.summary["settlement"], "passed", False),
            (self.summary["settlement"], "owned_pids", [123]),
            (self.summary["settlement"], "receipt_fsynced", False),
        ]:
            with self.subTest(key=key):
                previous = section[key]
                section[key] = value
                self.save()
                with self.assertRaises(ValueError):
                    self.audit()
                section[key] = previous


if __name__ == "__main__":
    unittest.main()
