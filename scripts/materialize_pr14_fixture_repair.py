#!/usr/bin/env python3
"""Materialize the exact PR14 stale-fixture repair without changing production code."""

from pathlib import Path

path = Path("tests/podman_command_execution.rs")
text = path.read_text(encoding="utf-8")

format_before = '''    let init_capable_script = format!(
        "if [ \\"${{1:-}}\\" = init ]; then exit 0; fi\\n{script}"
    );'''
format_after = '''    let init_capable_script = format!("if [ \\"${{1:-}}\\" = init ]; then exit 0; fi\\n{script}");'''
if text.count(format_before) != 1:
    raise SystemExit("init-capable formatting shape changed; refusing ambiguous repair")
text = text.replace(format_before, format_after, 1)

fn_start = text.index("fn run_command_at_cleans_up_when_container_start_fails()")
fn_end = text.index("\n#[test]", fn_start + 1)
section = text[fn_start:fn_end]

create_then_start = "         create:--name) printf 'fake-command-container-id\\\\n' ;;\\n  \\\\\n         start:*) exit 1 ;;"
create_inspect_start = "         create:--name) printf 'fake-command-container-id\\\\n' ;;\\n  \\\\\n         container:inspect) printf '%s\\\\n' '{}' ;;\\n  \\\\\n         start:*) exit 1 ;;"
if section.count(create_then_start) != 1:
    raise SystemExit("start-failure fixture command shape changed; refusing ambiguous repair")
section = section.replace(create_then_start, create_inspect_start, 1)

args_before = '''        log.display(),
        security_info_json(),
    );'''
args_after = '''        log.display(),
        security_info_json(),
        container_inspect_json("fake-command-container-id"),
    );'''
if section.count(args_before) != 1:
    raise SystemExit("start-failure fixture format arguments changed; refusing ambiguous repair")
section = section.replace(args_before, args_after, 1)

text = text[:fn_start] + section + text[fn_end:]
path.write_text(text, encoding="utf-8")
