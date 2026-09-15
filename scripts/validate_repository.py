#!/usr/bin/env python3
"""Validate repository policy without executing untrusted artifacts."""

from __future__ import annotations

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMA_FILES = (
    "schemas/analysis-request.schema.json",
    "schemas/evidence-bundle.schema.json",
    "schemas/application-service-request.schema.json",
    "schemas/application-service-lease.schema.json",
    "schemas/application-service-cleanup.schema.json",
    "schemas/command-execution-request.schema.json",
    "schemas/command-execution-result.schema.json",
    "schemas/isolation-policy.schema.json",
    "schemas/release-evidence.schema.json",
)
REQUIRED_FILES = (
    "AGENTS.md",
    "CHANGELOG.md",
    "CLAUDE.md",
    "LICENSE",
    "README.md",
    "RELEASE.md",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    "src/artifact_analysis/mod.rs",
    "src/artifact_analysis/contracts.rs",
    "src/artifact_analysis/ingestion.rs",
    "src/artifact_analysis/runtime.rs",
    "src/application_service/mod.rs",
    "src/sandbox_execution/bounded_command_execution.rs",
    "src/sandbox_execution/mod.rs",
    "src/infrastructure/mod.rs",
    "src/infrastructure/podman.rs",
    "src/main.rs",
    "docs/ARCHITECTURE.md",
    "docs/OPERABILITY.md",
    "docs/PRD.md",
    "docs/SECURITY.md",
    "docs/TEST_STRATEGY.md",
    "docs/THREAT_MODEL.md",
    "docs/TRD.md",
    "docs/product-technical-gap-baseline.md",
    "docs/contracts/consumer-contract.md",
    "docs/adr/README.md",
    "docs/adr/0001-product-authority-boundary.md",
    "docs/adr/0002-credential-free-default-deny.md",
    "docs/adr/0003-published-contract-consumption.md",
    "docs/adr/0004-truthful-capability-claims.md",
    "docs/adr/0005-sandbox-execution-context.md",
    "docs/adr/0006-isolated-application-service.md",
    "docs/adr/0007-bounded-command-execution-contract.md",
    "docs/adr/0008-podman-backed-command-execution-and-cli.md",
    "docs/doctoring/REFERENCES.md",
    "docs/doctoring/STANDARD_TRACEABILITY.md",
    *SCHEMA_FILES,
)
FORBIDDEN_DDD_PATHS = (
    "src/contracts.rs",
    "src/ingestion.rs",
    "src/runtime.rs",
    "src/sandbox_execution/podman.rs",
)
FORBIDDEN_PLACEHOLDERS = re.compile(r"\b(?:TBD|TODO|FIXME)\b")
FORBIDDEN_DATABASE_NAME = re.compile(
    r"\bCREATE\s+(?:TABLE|SCHEMA|TYPE)\s+(?:IF\s+NOT\s+EXISTS\s+)?"
    r"(?:[a-z][a-z0-9]*\.)?([a-z][a-z0-9]*)\b",
    re.IGNORECASE,
)
ADR_NAME = re.compile(r"^(\d{4})-.*\.md$")
ACTION_REFERENCE = re.compile(r"^[0-9a-f]{40}$")
SUPPORTING_CONTEXT_REFERENCE = re.compile(
    r"\bapplication_service\b|\bApplicationService[A-Za-z0-9_]*\b"
)


def rust_raw_string_end(source: str, index: int) -> int | None:
    """Return one-past-end for a Rust raw string token starting at ``index``."""

    if index > 0 and (source[index - 1].isalnum() or source[index - 1] == "_"):
        return None

    cursor = index
    if source.startswith(("br", "cr"), cursor):
        cursor += 2
    elif source.startswith("r", cursor):
        cursor += 1
    else:
        return None

    hash_start = cursor
    while cursor < len(source) and source[cursor] == "#":
        cursor += 1
    if cursor >= len(source) or source[cursor] != '"':
        return None

    hash_count = cursor - hash_start
    closing_delimiter = '"' + ("#" * hash_count)
    closing_index = source.find(closing_delimiter, cursor + 1)
    if closing_index < 0:
        return len(source)
    return closing_index + len(closing_delimiter)


def rust_code_without_comments_and_strings(source: str) -> str:
    """Return Rust source with comments and string contents removed.

    The DDD fitness rule is about compile-time dependencies. Documentation,
    diagnostic text, and compatibility notes may legitimately name a Supporting
    context and must not create a false dependency finding. Nested block comments
    and raw-string delimiters are handled because Rust permits both.
    """

    output: list[str] = []
    index = 0
    block_depth = 0
    in_string = False

    while index < len(source):
        if block_depth:
            if source.startswith("/*", index):
                block_depth += 1
                output.extend("  ")
                index += 2
            elif source.startswith("*/", index):
                block_depth -= 1
                output.extend("  ")
                index += 2
            else:
                output.append("\n" if source[index] == "\n" else " ")
                index += 1
            continue

        if in_string:
            character = source[index]
            if character == "\\" and index + 1 < len(source):
                output.extend("  ")
                index += 2
            elif character == '"':
                output.append(" ")
                in_string = False
                index += 1
            else:
                output.append("\n" if character == "\n" else " ")
                index += 1
            continue

        raw_string_end = rust_raw_string_end(source, index)
        if raw_string_end is not None:
            output.extend(
                "\n" if character == "\n" else " "
                for character in source[index:raw_string_end]
            )
            index = raw_string_end
            continue

        if source.startswith("//", index):
            while index < len(source) and source[index] != "\n":
                output.append(" ")
                index += 1
            continue
        if source.startswith("/*", index):
            block_depth = 1
            output.extend("  ")
            index += 2
            continue
        if source[index] == '"':
            in_string = True
            output.append(" ")
            index += 1
            continue

        output.append(source[index])
        index += 1

    return "".join(output)


def core_depends_on_application_service(source: str) -> bool:
    """Return whether executable Rust code references the Supporting context."""

    executable_source = rust_code_without_comments_and_strings(source)
    return SUPPORTING_CONTEXT_REFERENCE.search(executable_source) is not None


def main() -> int:
    """Return zero when all repository policy checks pass."""

    errors: list[str] = []

    for relative_path in REQUIRED_FILES:
        if not (ROOT / relative_path).is_file():
            errors.append(f"missing required file: {relative_path}")

    for relative_path in FORBIDDEN_DDD_PATHS:
        if (ROOT / relative_path).exists():
            errors.append(
                "DDD ownership regression: implementation is in the wrong bounded-context path: "
                f"{relative_path}"
            )

    sandbox_root = ROOT / "src/sandbox_execution"
    for path in sorted(sandbox_root.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        if core_depends_on_application_service(text):
            errors.append(
                "DDD dependency regression: Core sandbox_execution must not depend on "
                f"Supporting application_service: {path.relative_to(ROOT)}"
            )

    adr_numbers: dict[str, pathlib.Path] = {}
    adr_root = ROOT / "docs/adr"
    for path in sorted(adr_root.glob("*.md")):
        match = ADR_NAME.fullmatch(path.name)
        if match is None:
            continue
        number = match.group(1)
        existing = adr_numbers.get(number)
        if existing is not None:
            errors.append(
                "duplicate ADR identifier: "
                f"{number} in {existing.relative_to(ROOT)} and {path.relative_to(ROOT)}"
            )
        else:
            adr_numbers[number] = path

    for path in sorted(ROOT.rglob("*")):
        if not path.is_file() or ".git" in path.parts or "target" in path.parts:
            continue
        if path.suffix.lower() not in {
            ".md",
            ".json",
            ".py",
            ".rs",
            ".toml",
            ".yml",
            ".yaml",
        }:
            continue
        text = path.read_text(encoding="utf-8")
        if (
            path.resolve() != pathlib.Path(__file__).resolve()
            and FORBIDDEN_PLACEHOLDERS.search(text)
        ):
            errors.append(f"placeholder token found: {path.relative_to(ROOT)}")
        for match in FORBIDDEN_DATABASE_NAME.finditer(text):
            object_name = match.group(1)
            if "_" not in object_name:
                errors.append(
                    "database object must contain two or more words: "
                    f"{path.relative_to(ROOT)}:{object_name}"
                )

    for relative_path in SCHEMA_FILES:
        schema_path = ROOT / relative_path
        try:
            schema = json.loads(schema_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON schema {schema_path.name}: {exc}")
            continue
        if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
            errors.append(f"schema is not Draft 2020-12: {schema_path.name}")
        if schema.get("additionalProperties") is not False:
            errors.append(f"top-level schema must fail closed: {schema_path.name}")

    workflow_root = ROOT / ".github/workflows"
    workflow_paths = sorted(workflow_root.glob("*.yml")) + sorted(workflow_root.glob("*.yaml"))
    for workflow_path in workflow_paths:
        workflow = workflow_path.read_text(encoding="utf-8")
        uses_lines = [
            line.strip()
            for line in workflow.splitlines()
            if line.strip().startswith("uses:")
        ]
        for uses_line in uses_lines:
            if "@" not in uses_line:
                errors.append(
                    f"workflow action is unpinned in {workflow_path.name}: {uses_line}"
                )
                continue
            reference = uses_line.rsplit("@", maxsplit=1)[1].split()[0]
            if not ACTION_REFERENCE.fullmatch(reference):
                errors.append(
                    "workflow action is not pinned by commit SHA in "
                    f"{workflow_path.name}: {uses_line}"
                )

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print("repository policy validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
