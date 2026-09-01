#!/usr/bin/env python3
"""Explain and enforce complete LLVM source coverage."""

from __future__ import annotations

import argparse
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


def _uncovered_lines(data: dict[str, Any], filename: str) -> list[int]:
    """Return source lines whose function regions are never executed.

    LLVM can emit several regions for one source line, especially after macro
    expansion.  A line is therefore uncovered only when at least one function
    maps a region to it and no mapped function region records execution.
    """

    uncovered: set[int] = set()
    covered: set[int] = set()
    functions = data.get("functions")
    if not isinstance(functions, list):
        return []

    for function in functions:
        if not isinstance(function, dict):
            continue
        filenames = function.get("filenames")
        if not isinstance(filenames, list) or not filenames or str(filenames[0]) != filename:
            continue
        line_counts: dict[int, int] = {}
        regions = function.get("regions")
        if not isinstance(regions, list):
            continue
        for region in regions:
            if not isinstance(region, list) or len(region) < 5:
                continue
            line_start = int(region[0])
            line_end = int(region[2])
            execution_count = int(region[4])
            for line_number in range(line_start, line_end + 1):
                line_counts[line_number] = line_counts.get(line_number, 0) + execution_count
        for line_number, execution_count in line_counts.items():
            if execution_count == 0:
                uncovered.add(line_number)
            else:
                covered.add(line_number)

    return sorted(uncovered - covered)


def _parse_arguments() -> argparse.Namespace:
    """Parse the coverage evidence path and optional branch requirement."""

    parser = argparse.ArgumentParser()
    parser.add_argument("coverage_path", type=pathlib.Path)
    parser.add_argument("--require-branches", action="store_true")
    return parser.parse_args()


def main() -> int:
    """Print uncovered production locations and fail below complete coverage."""

    arguments = _parse_arguments()
    payload = json.loads(arguments.coverage_path.read_text(encoding="utf-8"))
    data_sets = payload.get("data")
    if not isinstance(data_sets, list) or len(data_sets) != 1:
        print("expected exactly one LLVM coverage data set", file=sys.stderr)
        return 2

    data = data_sets[0]
    failures: list[str] = []
    totals = data.get("totals", {})
    metric_names = ["lines", "functions", "regions"]
    if arguments.require_branches:
        metric_names.append("branches")

    for metric_name in metric_names:
        metric = totals.get(metric_name)
        if not isinstance(metric, dict):
            failures.append(f"missing coverage metric: {metric_name}")
            continue
        total, covered = _metric_counts(metric)
        print(f"{metric_name}: {covered}/{total}")
        if metric_name == "branches" and total == 0:
            failures.append("branch instrumentation produced zero branches")
        elif total != covered:
            failures.append(f"{metric_name} coverage is {covered}/{total}")

    for file_record in data.get("files", []):
        filename = str(file_record.get("filename", "<unknown>"))
        summary = file_record.get("summary", {})
        incomplete_metrics: list[str] = []
        for metric_name in metric_names:
            metric = summary.get(metric_name)
            if not isinstance(metric, dict):
                continue
            total, covered = _metric_counts(metric)
            if total != covered:
                incomplete_metrics.append(f"{metric_name}={covered}/{total}")
        if incomplete_metrics:
            print(f"incomplete file: {filename}: {', '.join(incomplete_metrics)}")
            missing_lines = _uncovered_lines(data, filename)
            if missing_lines:
                joined_lines = ", ".join(str(line_number) for line_number in missing_lines)
                print(f"uncovered lines: {filename}: {joined_lines}")

    if failures:
        print("; ".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
