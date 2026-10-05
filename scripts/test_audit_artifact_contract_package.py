"""Offline, synthetic source-package binding regressions; no publication."""

import hashlib
import importlib.util
import io
import json
import pathlib
import tarfile
import tempfile
import unittest
import subprocess
import sys
from unittest import mock
from contextlib import redirect_stdout
import runpy
import gzip
import zlib

SCRIPT = pathlib.Path(__file__).with_name("audit_artifact_contract_package.py")
SPEC = importlib.util.spec_from_file_location("artifact_package_audit", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class PackageAuditTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.archive = pathlib.Path(self.directory.name) / "candidate.crate"
        self.revision = "a" * 40
        self.expected = {"schemas/analysis-request.schema.json": b'{"field":"bounded"}'}
        self.prefix = "example-1.0.0"

    def pack(self, content=None):
        entries = dict(self.expected if content is None else content)
        entries[".cargo_vcs_info.json"] = json.dumps(
            {"git": {"sha1": self.revision}, "path_in_vcs": ""}
        ).encode()
        with tarfile.open(self.archive, "w:gz") as stream:
            for name, data in entries.items():
                member = tarfile.TarInfo(f"{self.prefix}/{name}")
                member.size = len(data)
                stream.addfile(member, io.BytesIO(data))

    def test_archive_admission_rejects_extra_unsafe_duplicate_and_link_members(self):
        for kind in ("unsafe", "duplicate", "symlink"):
            with self.subTest(kind=kind):
                self.pack()
                with tarfile.open(self.archive, "r:gz") as old:
                    entries = [
                        (member, old.extractfile(member).read())
                        for member in old.getmembers()
                    ]
                with tarfile.open(self.archive, "w:gz") as stream:
                    for member, data in entries:
                        stream.addfile(member, io.BytesIO(data))
                    name = (
                        "../outside"
                        if kind == "unsafe"
                        else f"{self.prefix}/schemas/analysis-request.schema.json"
                        if kind == "duplicate"
                        else f"{self.prefix}/link"
                    )
                    extra = tarfile.TarInfo(name)
                    if kind == "symlink":
                        extra.type = tarfile.SYMTYPE
                        extra.linkname = "schemas/analysis-request.schema.json"
                    stream.addfile(extra)
                with self.assertRaises(AUDIT.AuditError):
                    AUDIT.audit_package(
                        self.archive, self.revision, self.prefix, self.expected
                    )

    def test_rejects_missing_contract_and_invalid_revision_metadata(self):
        self.pack({})
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)
        for metadata in (
            b'{"git":{"sha1":"wrong"}}',
            b"[]",
            json.dumps(
                {"git": {"sha1": self.revision, "dirty": True}, "path_in_vcs": ""}
            ).encode(),
            b'{"git":{},"git":{}}',
        ):
            with self.subTest(metadata=metadata):
                self.pack_metadata(metadata)
                with self.assertRaises(AUDIT.AuditError):
                    AUDIT.audit_package(
                        self.archive, self.revision, self.prefix, self.expected
                    )

    def pack_metadata(self, metadata):
        with tarfile.open(self.archive, "w:gz") as stream:
            for name, data in {
                **self.expected,
                ".cargo_vcs_info.json": metadata,
            }.items():
                member = tarfile.TarInfo(f"{self.prefix}/{name}")
                member.size = len(data)
                stream.addfile(member, io.BytesIO(data))

    def test_cli_binds_real_git_commit_and_returns_fixed_failure_json(self):
        repo = pathlib.Path(self.directory.name) / "repo"
        repo.mkdir()
        subprocess.run(["git", "init", "-q", str(repo)], check=True)
        expected = {}
        for name in AUDIT.CONTRACT_PATHS:
            path = repo / ("Cargo.toml" if name == "Cargo.toml.orig" else name)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"contract bytes")
            expected[name] = path.read_bytes()
        subprocess.run(["git", "add", "."], cwd=repo, check=True)
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "core.hooksPath=/dev/null",
                "commit",
                "-qm",
                "fixture",
            ],
            cwd=repo,
            check=True,
        )
        revision = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=repo, text=True
        ).strip()
        self.revision = revision
        self.expected = expected
        self.pack()
        command = [
            sys.executable,
            str(SCRIPT),
            "--archive",
            str(self.archive),
            "--repository",
            str(repo),
            "--revision",
            revision,
            "--prefix",
            self.prefix,
        ]
        result = subprocess.run(
            command, cwd=self.directory.name, capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        receipt = json.loads(result.stdout)
        self.assertEqual(len(receipt["files"]), len(AUDIT.CONTRACT_PATHS))
        self.assertFalse(receipt["release_allowed"])
        self.pack({**expected, next(iter(expected)): b"altered"})
        result = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(
            json.loads(result.stdout),
            {"ok": False, "release_allowed": False, "error": "PACKAGE_AUDIT_FAILED"},
        )
        self.assertEqual(result.stderr, "")

    def test_rejects_empty_expected_and_invalid_revision(self):
        self.pack()
        for revision, prefix, expected in [
            ("bad", self.prefix, self.expected),
            (self.revision, "../bad", self.expected),
            (self.revision, self.prefix, {}),
        ]:
            with self.assertRaises(AUDIT.AuditError):
                AUDIT.audit_package(self.archive, revision, prefix, expected)

    def test_bounded_archive_and_expected_input_rejections(self):
        self.pack()
        for constant, value in [
            ("MAX_ARCHIVE_BYTES", 1),
            ("MAX_EXPANDED_BYTES", 1),
            ("MAX_MEMBERS", 1),
            ("MAX_METADATA_BYTES", 1),
        ]:
            with (
                self.subTest(constant=constant),
                mock.patch.object(AUDIT, constant, value),
            ):
                with self.assertRaises(AUDIT.AuditError):
                    AUDIT.audit_package(
                        self.archive, self.revision, self.prefix, self.expected
                    )
        for expected in ({"../file": b"x"}, {"file": "not bytes"}):
            with self.assertRaises(AUDIT.AuditError):
                AUDIT.audit_package(self.archive, self.revision, self.prefix, expected)
        self.pack_metadata(b"{invalid json")
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)
        self.pack_metadata(b"\xff")
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)
        self.pack({**self.expected, next(iter(self.expected)): b"altered"})
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)

    def test_regular_reader_refuses_absent_or_short_reads(self):
        member = tarfile.TarInfo("fixture")
        member.size = 3
        with self.assertRaises(AUDIT.AuditError):
            AUDIT._read_regular(
                mock.Mock(extractfile=mock.Mock(return_value=None)), member
            )
        with self.assertRaises(AUDIT.AuditError):
            AUDIT._read_regular(
                mock.Mock(extractfile=mock.Mock(return_value=io.BytesIO(b"x"))), member
            )

    def test_duplicate_valid_metadata_cannot_hide_behind_other_shape_failures(self):
        metadata = (
            '{"git":{"sha1":"' + self.revision + '"},'
            '"git":{"sha1":"' + self.revision + '"},"path_in_vcs":""}'
        ).encode()
        self.pack_metadata(metadata)
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)

    def test_expanded_tar_padding_is_bounded_independently_of_member_sizes(self):
        self.pack()
        raw = gzip.decompress(self.archive.read_bytes())
        self.archive.write_bytes(gzip.compress(raw))
        with mock.patch.object(AUDIT, "MAX_EXPANDED_BYTES", 1024):
            with self.assertRaises(AUDIT.AuditError):
                AUDIT.audit_package(
                    self.archive, self.revision, self.prefix, self.expected
                )

    def test_plain_tar_is_supported_without_compression(self):
        self.pack()
        self.archive.write_bytes(gzip.decompress(self.archive.read_bytes()))
        result = AUDIT.audit_package(
            self.archive, self.revision, self.prefix, self.expected
        )
        self.assertTrue(result["contract_files_match"])
        self.assertFalse(result["release_allowed"])

    def test_metadata_must_exist(self):
        with tarfile.open(self.archive, "w:gz") as stream:
            for name, payload in self.expected.items():
                member = tarfile.TarInfo(f"{self.prefix}/{name}")
                member.size = len(payload)
                stream.addfile(member, io.BytesIO(payload))
        with self.assertRaises(AUDIT.AuditError):
            AUDIT.audit_package(self.archive, self.revision, self.prefix, self.expected)

    def test_git_source_and_main_paths_are_bounded_and_fail_closed(self):
        outputs = {name: b"source" for name in AUDIT.CONTRACT_PATHS}

        def git_result(command, **kwargs):
            self.assertEqual(command[:3], ["git", "-C", "repo"])
            self.assertTrue(kwargs["check"])
            return subprocess.CompletedProcess(command, 0, b"source", b"")

        with mock.patch.object(AUDIT.subprocess, "run", side_effect=git_result):
            self.assertEqual(AUDIT._git_contracts("repo", self.revision), outputs)
        with (
            mock.patch.object(AUDIT.subprocess, "run", side_effect=git_result),
            mock.patch.object(AUDIT, "MAX_EXPANDED_BYTES", 1),
        ):
            with self.assertRaises(AUDIT.AuditError):
                AUDIT._git_contracts("repo", self.revision)
        args = [
            "--archive",
            str(self.archive),
            "--repository",
            "repo",
            "--revision",
            self.revision,
            "--prefix",
            self.prefix,
        ]
        self.pack()
        output = io.StringIO()
        with (
            mock.patch.object(AUDIT, "_git_contracts", return_value=self.expected),
            redirect_stdout(output),
        ):
            self.assertEqual(AUDIT.main(args), 0)
        self.assertFalse(json.loads(output.getvalue())["release_allowed"])
        output = io.StringIO()
        with (
            mock.patch.object(
                AUDIT, "_git_contracts", side_effect=OSError("private detail")
            ),
            redirect_stdout(output),
        ):
            self.assertEqual(AUDIT.main(args), 2)
        self.assertEqual(json.loads(output.getvalue())["error"], "PACKAGE_AUDIT_FAILED")

        output = io.StringIO()
        with (
            mock.patch.object(AUDIT, "_git_contracts", return_value=self.expected),
            mock.patch.object(
                AUDIT.gzip.GzipFile,
                "read",
                side_effect=zlib.error("invalid compression"),
            ),
            redirect_stdout(output),
        ):
            with mock.patch.object(
                AUDIT.pathlib.Path, "open", return_value=io.BytesIO(b"\x1f\x8b")
            ):
                self.assertEqual(AUDIT.main(args), 2)
        self.assertEqual(json.loads(output.getvalue())["error"], "PACKAGE_AUDIT_FAILED")

    def test_cli_entrypoint_reports_fixed_failure_without_git_call_on_bad_identity(
        self,
    ):
        output = io.StringIO()
        args = [
            str(SCRIPT),
            "--archive",
            "missing",
            "--repository",
            "repo",
            "--revision",
            "bad",
            "--prefix",
            self.prefix,
        ]
        with mock.patch.object(sys, "argv", args), redirect_stdout(output):
            with self.assertRaises(SystemExit) as exit_result:
                runpy.run_path(str(SCRIPT), run_name="__main__")
        self.assertEqual(exit_result.exception.code, 2)
        self.assertFalse(json.loads(output.getvalue())["release_allowed"])

    def test_matching_bytes_report_candidate_binding_not_release(self):
        self.pack()
        result = AUDIT.audit_package(
            self.archive, self.revision, self.prefix, self.expected
        )
        self.assertTrue(result["contract_files_match"])
        self.assertFalse(result["release_allowed"])
        self.assertEqual(result["source_sha"], self.revision)
        self.assertEqual(
            result["archive_sha256"],
            hashlib.sha256(self.archive.read_bytes()).hexdigest(),
        )
        self.assertEqual(
            result["files"][next(iter(self.expected))]["sha256"],
            hashlib.sha256(next(iter(self.expected.values()))).hexdigest(),
        )


if __name__ == "__main__":
    unittest.main()
