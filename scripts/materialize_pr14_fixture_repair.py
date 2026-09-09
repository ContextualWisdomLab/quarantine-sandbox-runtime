#!/usr/bin/env python3
"""Materialize the exact PR14 stale-fixture repair without changing production code."""

from pathlib import Path

path = Path("tests/podman_command_execution.rs")
lines = path.read_text(encoding="utf-8").splitlines(keepends=True)

function_marker = "fn run_command_at_cleans_up_when_container_start_fails()"
start_index = next((index for index, line in enumerate(lines) if function_marker in line), None)
if start_index is None:
    raise SystemExit("start-failure fixture missing; refusing repair")

end_index = next(
    (index for index in range(start_index + 1, len(lines)) if lines[index].startswith("#[test]")),
    len(lines),
)
section = lines[start_index:end_index]

start_matches = [index for index, line in enumerate(section) if "start:*) exit 1 ;;" in line]
if len(start_matches) != 1:
    raise SystemExit("start-failure command shape changed; refusing ambiguous repair")
start_line_index = start_index + start_matches[0]
lines.insert(
    start_line_index,
    "         container:inspect) printf '%s\\\\n' '{}' ;;\\n  \\\n",
)
end_index += 1

security_matches = [
    index
    for index in range(start_index, end_index)
    if lines[index].strip() == "security_info_json(),"
]
if len(security_matches) != 1:
    raise SystemExit("start-failure format arguments changed; refusing ambiguous repair")
security_line_index = security_matches[0]
indent = lines[security_line_index][: len(lines[security_line_index]) - len(lines[security_line_index].lstrip())]
lines.insert(
    security_line_index + 1,
    f'{indent}container_inspect_json("fake-command-container-id"),\n',
)

path.write_text("".join(lines), encoding="utf-8")
