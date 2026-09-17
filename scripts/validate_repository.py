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
    "schemas/isolation-policy.schema.json",
)
REQUIRED_FILES = (
    "AGENTS.md",
    "CHANGELOG.md",
    "CLAUDE.md",
    "LICENSE",
    "README.md",
    "src/artifact_analysis/mod.rs",
    "src/artifact_analysis/contracts.rs",
    "src/artifact_analysis/ingestion.rs",
    "src/artifact_analysis/runtime.rs",
    "src/application_service/mod.rs",
    "src/sandbox_execution/mod.rs",
    "src/infrastructure/mod.rs",
    "src/infrastructure/podman.rs",
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
BLOCK_SCALAR_START = re.compile(
    r"^\s*(?:-\s*)?(?:[A-Za-z_][A-Za-z0-9_.-]*|\"[^\"]+\"|'[^']+')"
    r"\s*:\s*[>|](?:[1-9]?[+-]?|[+-]?[1-9]?)\s*$"
)


def _normalize_workflow_mapping_key(raw_key: str) -> str:
    """Normalize simple YAML mapping keys and reject ambiguous complex-key syntax."""

    key = raw_key.strip()
    if key.startswith("-"):
        key = key[1:].strip()
    if key.startswith("?"):
        raise ValueError(f"unsupported explicit YAML mapping key: {key}")
    if key.startswith(("!", "&", "*")):
        raise ValueError(f"unsupported complex YAML mapping key: {key}")
    if key.startswith('"'):
        if not key.endswith('"'):
            raise ValueError(f"unsupported quoted YAML mapping key: {key}")
        try:
            decoded = json.loads(key)
        except json.JSONDecodeError as exc:
            raise ValueError(f"unsupported quoted YAML mapping key: {key}") from exc
        if not isinstance(decoded, str):
            raise ValueError(f"unsupported quoted YAML mapping key: {key}")
        return decoded
    if key.startswith("'"):
        if not key.endswith("'"):
            raise ValueError(f"unsupported quoted YAML mapping key: {key}")
        return key[1:-1].replace("''", "'")
    return key


def _workflow_uses_targets(workflow: str) -> list[str]:
    """Return admitted workflow `uses` values from block or YAML flow mappings."""

    targets: list[str] = []
    block_scalar_indent: int | None = None
    flow_mapping_depth = 0
    in_expression = False

    for raw_line in workflow.splitlines():
        if not raw_line.strip():
            continue

        indent = len(raw_line) - len(raw_line.lstrip(" "))
        if block_scalar_indent is not None:
            if indent > block_scalar_indent:
                continue
            block_scalar_indent = None

        in_single_quote = False
        in_double_quote = False
        escaped = False
        curly_depth = flow_mapping_depth
        comment_at = len(raw_line)
        separators = [0]
        colons: list[int] = []
        index = 0

        while index < len(raw_line):
            character = raw_line[index]
            if in_expression:
                if raw_line.startswith("}}", index):
                    in_expression = False
                    index += 2
                else:
                    index += 1
                continue
            if in_single_quote:
                if character == "'":
                    if index + 1 < len(raw_line) and raw_line[index + 1] == "'":
                        index += 2
                        continue
                    in_single_quote = False
                index += 1
                continue
            if in_double_quote:
                if escaped:
                    escaped = False
                elif character == "\\":
                    escaped = True
                elif character == '"':
                    in_double_quote = False
                index += 1
                continue
            if raw_line.startswith("${{", index):
                in_expression = True
                index += 3
                continue
            if character == "#" and (index == 0 or raw_line[index - 1].isspace()):
                comment_at = index
                break
            if character == "'":
                in_single_quote = True
            elif character == '"':
                in_double_quote = True
            elif character == "{":
                curly_depth += 1
                separators.append(index + 1)
            elif character == "}":
                curly_depth = max(0, curly_depth - 1)
            elif character == "," and curly_depth > 0:
                separators.append(index + 1)
            elif character == ":":
                colons.append(index)
            index += 1

        visible = raw_line[:comment_at]
        if BLOCK_SCALAR_START.fullmatch(visible):
            block_scalar_indent = indent

        structural_line = visible.lstrip()
        if structural_line.startswith("-"):
            structural_line = structural_line[1:].lstrip()
        if structural_line.startswith("?"):
            raise ValueError(
                f"unsupported explicit YAML mapping key: {structural_line}"
            )

        for colon in colons:
            if colon >= comment_at:
                continue
            segment_start = max(
                separator for separator in separators if separator <= colon
            )
            key = _normalize_workflow_mapping_key(
                visible[segment_start:colon]
            )
            if key != "uses":
                continue

            remaining = visible[colon + 1 :].lstrip()
            if not remaining:
                targets.append("")
                continue
            if remaining[0] not in {"'", '"'}:
                targets.append(re.split(r"[\s,}]", remaining, maxsplit=1)[0])
                continue

            quote = remaining[0]
            value: list[str] = []
            cursor = 1
            while cursor < len(remaining):
                character = remaining[cursor]
                if quote == "'" and character == "'":
                    if cursor + 1 < len(remaining) and remaining[cursor + 1] == "'":
                        value.append("'")
                        cursor += 2
                        continue
                    break
                if quote == '"' and character == "\\":
                    if cursor + 1 >= len(remaining):
                        value.append("\\")
                        break
                    value.append(remaining[cursor + 1])
                    cursor += 2
                    continue
                if character == quote:
                    break
                value.append(character)
                cursor += 1
            targets.append("".join(value))

        flow_mapping_depth = curly_depth

    return targets


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
        if "application_service" in text or "ApplicationService" in text:
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
    try:
        workflow_paths = sorted(
            path
            for path in workflow_root.iterdir()
            if path.is_file() and path.suffix.lower() in {".yml", ".yaml"}
        )
    except OSError as exc:
        errors.append(f"workflow directory is unreadable: {exc}")
        workflow_paths = []

    if not workflow_paths:
        errors.append("workflow directory contains no .yml or .yaml workflow files")

    for workflow_path in workflow_paths:
        try:
            workflow = workflow_path.read_text(encoding="utf-8")
        except OSError as exc:
            errors.append(
                f"workflow file is unreadable: {workflow_path.relative_to(ROOT)}: {exc}"
            )
            continue
        try:
            uses_targets = _workflow_uses_targets(workflow)
        except ValueError as exc:
            errors.append(
                "workflow dependency syntax is unsupported for fail-closed validation: "
                f"{workflow_path.relative_to(ROOT)}: {exc}"
            )
            continue
        for uses_target in uses_targets:
            if uses_target.startswith("$/"):
                continue
            if "@" not in uses_target:
                errors.append(f"workflow action is unpinned: {uses_target}")
                continue
            reference = uses_target.rsplit("@", maxsplit=1)[1]
            if not re.fullmatch(r"[0-9a-f]{40}", reference):
                errors.append(
                    f"workflow action is not pinned by commit SHA: {uses_target}"
                )

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print("repository policy validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
