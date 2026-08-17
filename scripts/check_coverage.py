#!/usr/bin/env python3
"""Explain and enforce complete LLVM source coverage."""

from __future__ import annotations

import json
import pathlib
import sys
from typing import Any


def _metric_counts(metric: dict[str, Any]) -> tuple[int, int]:
    """Return total and covered counts from an LLVM coverage metric."""

    total = int(metric.get("count", 0))
    if "covered" in metric:
        return total, int(metric["covered"])
    if "notcovered" in metric:
        return total, total - int(metric["notcovered"])
    raise ValueError(f"coverage metric has no covered/notcovered count: {metric}")


def main() -> int:
    """Print uncovered production locations and fail below complete coverage."""

    if len(sys.argv) != 2:
        print("usage: check_coverage.py <llvm-coverage.json>", file=sys.stderr)
        return 2

    coverage_path = pathlib.Path(sys.argv[1])
    payload = json.loads(coverage_path.read_text(encoding="utf-8"))
    data_sets = payload.get("data")
    if not isinstance(data_sets, list) or len(data_sets) != 1:
        print("expected exactly one LLVM coverage data set", file=sys.stderr)
        return 2

    data = data_sets[0]
    failures: list[str] = []
    totals = data.get("totals", {})
    for metric_name in ("lines", "functions", "regions"):
        metric = totals.get(metric_name)
        if not isinstance(metric, dict):
            failures.append(f"missing coverage metric: {metric_name}")
            continue
        total, covered = _metric_counts(metric)
        print(f"{metric_name}: {covered}/{total}")
        if total != covered:
            failures.append(f"{metric_name} coverage is {covered}/{total}")

    for file_record in data.get("files", []):
        filename = str(file_record.get("filename", "<unknown>"))
        uncovered_lines = sorted(
            {
                int(segment[0])
                for segment in file_record.get("segments", [])
                if len(segment) >= 6
                and int(segment[2]) == 0
                and bool(segment[3])
                and bool(segment[4])
                and not bool(segment[5])
            }
        )
        if uncovered_lines:
            print(
                f"uncovered region entries: {filename}:"
                + ",".join(str(line) for line in uncovered_lines)
            )

    if failures:
        print("; ".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
