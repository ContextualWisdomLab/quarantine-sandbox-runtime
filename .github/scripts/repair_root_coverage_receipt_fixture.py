from pathlib import Path

path = Path("tests/root_coverage_edges.rs")
text = path.read_text()
old_name = "malformed_identifier_cleanup_failure"
new_name = "malformed_identifier_without_receipt"
if old_name not in text:
    raise SystemExit("stale malformed-identifier fixture token not found")
text = text.replace(old_name, new_name)
old_rm = '  rm:*) if [ "$failure" = malformed_identifier_without_receipt ]; then exit 17; else :; fi ;;'
if text.count(old_rm) != 1:
    raise SystemExit("stale generated-name cleanup fixture branch not found")
text = text.replace(old_rm, '  rm:*) : ;;', 1)
path.write_text(text)
