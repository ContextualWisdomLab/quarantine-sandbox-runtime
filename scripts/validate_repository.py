#!/usr/bin/env python3
"""Validate repository policy without executing untrusted artifacts."""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REQUIRED_FILES = (
    "AGENTS.md",
    "CHANGELOG.md",
    "CLAUDE.md",
    "LICENSE",
    "README.md",
    "docs/ARCHITECTURE.md",
    "docs/OPERABILITY.md",
    "docs/PRD.md",
    "docs/SECURITY.md",
    "docs/TEST_STRATEGY.md",
    "docs/THREAT_MODEL.md",
    "docs/TRD.md",
    "docs/doctoring/REFERENCES.md",
    "docs/doctoring/STANDARD_TRACEABILITY.md",
    "schemas/analysis-request.schema.json",
    "schemas/evidence-bundle.schema.json",
)
FORBIDDEN_PLACEHOLDERS = re.compile(r"\b(?:TBD|TODO|FIXME)\b")
FORBIDDEN_DATABASE_NAME = re.compile(
    r"\bCREATE\s+(?:TABLE|SCHEMA|TYPE)\s+(?:IF\s+NOT\s+EXISTS\s+)?"
    r"(?:[a-z][a-z0-9]*\.)?([a-z][a-z0-9]*)\b",
    re.IGNORECASE,
)


def main() -> int:
    """Return zero when all repository policy checks pass."""

    errors: list[str] = []

    for relative_path in REQUIRED_FILES:
        if not (ROOT / relative_path).is_file():
            errors.append(f"missing required file: {relative_path}")

    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or ".git" in path.parts or "target" in path.parts:
            continue
        if path.suffix.lower() not in {".md", ".json", ".py", ".rs", ".toml", ".yml", ".yaml"}:
            continue
        text = path.read_text(encoding="utf-8")
        if path.resolve() != pathlib.Path(__file__).resolve() and FORBIDDEN_PLACEHOLDERS.search(text):
            errors.append(f"placeholder token found: {path.relative_to(ROOT)}")
        for match in FORBIDDEN_DATABASE_NAME.finditer(text):
            object_name = match.group(1)
            if "_" not in object_name:
                errors.append(
                    f"database object must contain two or more words: "
                    f"{path.relative_to(ROOT)}:{object_name}"
                )

    for schema_path in (
        ROOT / "schemas/analysis-request.schema.json",
        ROOT / "schemas/evidence-bundle.schema.json",
    ):
        try:
            schema = json.loads(schema_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON schema {schema_path.name}: {exc}")
            continue
        if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            errors.append(f"schema is not Draft 2020-12: {schema_path.name}")
        if schema.get("additionalProperties") is not False:
            errors.append(f"top-level schema must fail closed: {schema_path.name}")

    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    uses_lines = [
        line.strip()
        for line in workflow.splitlines()
        if line.strip().startswith("uses:")
    ]
    for uses_line in uses_lines:
        if "@" not in uses_line:
            errors.append(f"workflow action is unpinned: {uses_line}")
            continue
        reference = uses_line.rsplit("@", maxsplit=1)[1].split()[0]
        if not re.fullmatch(r"[0-9a-f]{40}", reference):
            errors.append(f"workflow action is not pinned by commit SHA: {uses_line}")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print("repository policy validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
