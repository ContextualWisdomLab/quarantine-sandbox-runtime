"""Read-only candidate contract-byte binding; never authorize release.

This tool does not execute or extract package content. Git source and revision
are caller-selected evidence, not authenticated publisher or release authority.
"""

import argparse
import hashlib
import gzip
import zlib
import io
import json
import pathlib
import re
import subprocess
import tarfile

CONTRACT_PATHS = (
    "Cargo.toml.orig",
    "schemas/analysis-request.schema.json",
    "schemas/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json",
    "docs/contracts/cwl_artifact_analysis_contract_vocabulary_1_0_0.md",
    "tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json",
    "src/artifact_analysis/contracts.rs",
)
MAX_ARCHIVE_BYTES = 16 * 1024 * 1024
MAX_EXPANDED_BYTES = 64 * 1024 * 1024
MAX_MEMBERS = 4096
MAX_METADATA_BYTES = 16 * 1024


class AuditError(ValueError):
    """Candidate package failed a bounded source-binding check."""


def _validate_identity(revision, prefix, expected):
    if (
        not isinstance(revision, str)
        or re.fullmatch(r"[0-9a-f]{40}", revision) is None
        or not isinstance(prefix, str)
        or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,127}", prefix) is None
        or not isinstance(expected, dict)
        or not expected
    ):
        raise AuditError("invalid audit identity")
    for name, source in expected.items():
        if (
            not isinstance(name, str)
            or not isinstance(source, bytes)
            or any(part in ("", ".", "..") for part in name.split("/"))
            or "\\" in name
        ):
            raise AuditError("invalid expected contract")


def _unique_object(pairs):
    obj = {}
    for key, value in pairs:
        if key in obj:
            raise AuditError("duplicate metadata key")
        obj[key] = value
    return obj


def _read_regular(stream, member):
    entry = stream.extractfile(member)
    if entry is None:
        raise AuditError("unreadable contract file")
    with entry:
        data = entry.read(member.size + 1)
    if len(data) != member.size:
        raise AuditError("truncated member")
    return data


def audit_package(archive, revision, prefix, expected):
    """Bind supplied nonempty contract bytes to one bounded regular-file tar."""
    _validate_identity(revision, prefix, expected)
    with pathlib.Path(archive).open("rb") as source:
        data = source.read(MAX_ARCHIVE_BYTES + 1)
    if len(data) > MAX_ARCHIVE_BYTES:
        raise AuditError("archive byte limit exceeded")
    if data.startswith(b"\x1f\x8b"):
        with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
            raw_tar = compressed.read(MAX_EXPANDED_BYTES + 1)
    else:
        raw_tar = data
    if len(raw_tar) > MAX_EXPANDED_BYTES:
        raise AuditError("expanded tar byte limit exceeded")
    files = {}
    with tarfile.open(fileobj=io.BytesIO(raw_tar), mode="r:") as stream:
        members = {}
        total = 0
        for member in stream:
            parts = member.name.split("/")
            if (
                not member.isfile()
                or member.sparse is not None
                or len(parts) < 2
                or parts[0] != prefix
                or any(part in ("", ".", "..") for part in parts)
                or "\\" in member.name
                or member.name in members
            ):
                raise AuditError("unsupported archive member")
            total += member.size
            if (
                member.size < 0
                or total > MAX_EXPANDED_BYTES
                or len(members) >= MAX_MEMBERS
            ):
                raise AuditError("expanded archive limit exceeded")
            members[member.name] = member
        for name, source in expected.items():
            member = members.get(f"{prefix}/{name}")
            if member is None:
                raise AuditError("missing contract file")
            payload = _read_regular(stream, member)
            if payload != source:
                raise AuditError("contract file mismatch")
            files[name] = {
                "sha256": hashlib.sha256(payload).hexdigest(),
                "bytes": len(payload),
            }
        member = members.get(f"{prefix}/.cargo_vcs_info.json")
        if member is None or member.size > MAX_METADATA_BYTES:
            raise AuditError("missing or oversized revision metadata")
        try:
            vcs = json.loads(
                _read_regular(stream, member).decode("utf-8"),
                object_pairs_hook=_unique_object,
            )
        except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
            raise AuditError("invalid revision metadata") from error
        if (
            not isinstance(vcs, dict)
            or not isinstance(vcs.get("git"), dict)
            or vcs["git"].get("sha1") != revision
            or vcs["git"].get("dirty", False) is not False
            or vcs.get("path_in_vcs") != ""
        ):
            raise AuditError("source revision mismatch or dirty package")
    return {
        "ok": True,
        "contract_files_match": True,
        "release_allowed": False,
        "source_sha": revision,
        "archive_sha256": hashlib.sha256(data).hexdigest(),
        "files": files,
        "scope": "candidate contract-byte binding only",
    }


def _git_contracts(repository, revision):
    expected = {}
    for name in CONTRACT_PATHS:
        source_name = "Cargo.toml" if name == "Cargo.toml.orig" else name
        result = subprocess.run(
            ["git", "-C", str(repository), "show", f"{revision}:{source_name}"],
            capture_output=True,
            check=True,
            timeout=10,
        )
        if len(result.stdout) > MAX_EXPANDED_BYTES:
            raise AuditError("source contract byte limit exceeded")
        expected[name] = result.stdout
    return expected


def main(argv=None):
    """Print one non-secret JSON receipt; failures never authorize release."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--prefix", required=True)
    args = parser.parse_args(argv)
    try:
        _validate_identity(args.revision, args.prefix, {"identity": b""})
        expected = _git_contracts(args.repository, args.revision)
        receipt = audit_package(args.archive, args.revision, args.prefix, expected)
    except (
        AuditError,
        OSError,
        tarfile.TarError,
        zlib.error,
        subprocess.SubprocessError,
        EOFError,
        RecursionError,
    ):
        print(
            json.dumps(
                {"ok": False, "release_allowed": False, "error": "PACKAGE_AUDIT_FAILED"}
            )
        )
        return 2
    print(json.dumps(receipt, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
