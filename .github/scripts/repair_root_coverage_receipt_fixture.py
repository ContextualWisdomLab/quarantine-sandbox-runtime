from pathlib import Path

path = Path("tests/root_coverage_edges.rs")
text = path.read_text()
old_name = "malformed_identifier_cleanup_failure"
new_name = "malformed_identifier_without_receipt"
if text.count(old_name) != 2:
    raise SystemExit("expected two stale malformed-identifier shell fixture tokens")
text = text.replace(old_name, new_name)
path.write_text(text)
