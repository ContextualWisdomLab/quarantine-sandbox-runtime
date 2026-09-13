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


def _canonical_region_counts(data: dict[str, Any]) -> dict[str, tuple[int, int]]:
    """Return source-region counts after combining duplicate codegen instances.

    ``llvm-cov export`` may contain multiple function records for the same
    source region when one Rust function is instantiated into more than one
    instrumented object. Source coverage is complete when any such instance
    executes that exact source region. The file-summary region count remains
    the denominator authority; a coordinate mismatch fails closed rather than
    silently shrinking it.
    """

    file_region_totals: dict[str, int] = {}
    for file_record in data.get("files", []):
        if not isinstance(file_record, dict):
            continue
        filename = str(file_record.get("filename", ""))
        summary = file_record.get("summary")
        if not filename or not isinstance(summary, dict):
            continue
        region_metric = summary.get("regions")
        if not isinstance(region_metric, dict):
            continue
        total, _ = _metric_counts(region_metric)
        file_region_totals[filename] = total

    source_regions: dict[str, dict[tuple[int, int, int, int, int], int]] = {
        filename: {} for filename in file_region_totals
    }
    functions = data.get("functions")
    if not isinstance(functions, list):
        functions = []

    for function in functions:
        if not isinstance(function, dict):
            continue
        filenames = function.get("filenames")
        regions = function.get("regions")
        if not isinstance(filenames, list) or not isinstance(regions, list):
            continue
        for region in regions:
            if not isinstance(region, list) or len(region) < 8:
                continue
            file_id = int(region[5])
            if file_id < 0 or file_id >= len(filenames):
                continue
            filename = str(filenames[file_id])
            if filename not in source_regions:
                continue
            key = (
                int(region[0]),
                int(region[1]),
                int(region[2]),
                int(region[3]),
                int(region[7]),
            )
            execution_count = int(region[4])
            previous_count = source_regions[filename].get(key, 0)
            source_regions[filename][key] = max(previous_count, execution_count)

    counts: dict[str, tuple[int, int]] = {}
    for filename, regions_by_coordinate in source_regions.items():
        canonical_total = len(regions_by_coordinate)
        raw_total = file_region_totals[filename]
        if canonical_total != raw_total:
            raise ValueError(
                "canonical source-region count disagrees with LLVM file summary: "
                f"{filename}: canonical={canonical_total}, raw={raw_total}"
            )
        canonical_covered = sum(
            execution_count > 0 for execution_count in regions_by_coordinate.values()
        )
        counts[filename] = (canonical_total, canonical_covered)
    return counts


def _uncovered_lines(data: dict[str, Any], filename: str) -> list[int]:
    """Return source lines whose function regions are never executed.

    LLVM can emit several regions for one source line, especially after macro
    expansion. Each region carries its own file id into the function's
    ``filenames`` table, so attribution must follow that id rather than assume
    every region belongs to the first filename. A line is uncovered only when
    at least one region for the requested file maps to it and no such region
    records execution.
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
        if not isinstance(filenames, list) or not filenames:
            continue
        line_counts: dict[int, int] = {}
        regions = function.get("regions")
        if not isinstance(regions, list):
            continue
        for region in regions:
            if not isinstance(region, list) or len(region) < 6:
                continue
            file_id = int(region[5])
            if file_id < 0 or file_id >= len(filenames) or str(filenames[file_id]) != filename:
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


def _uncovered_segment_starts(file_record: dict[str, Any]) -> list[tuple[int, int]]:
    """Return zero-count counted region-entry starts from one LLVM file record.

    LLVM's file-level ``segments`` surface is already scoped to the file and is
    therefore a useful diagnostic when function-region attribution cannot name
    an uncovered source line. Gap regions and non-counted boundary markers are
    excluded so the output identifies executable zero-count region entries
    rather than formatting or expansion boundaries.
    """

    locations: set[tuple[int, int]] = set()
    segments = file_record.get("segments")
    if not isinstance(segments, list):
        return []
    for segment in segments:
        if not isinstance(segment, list) or len(segment) < 6:
            continue
        line_number = int(segment[0])
        column_number = int(segment[1])
        execution_count = int(segment[2])
        has_count = bool(segment[3])
        is_region_entry = bool(segment[4])
        is_gap_region = bool(segment[5])
        if has_count and is_region_entry and not is_gap_region and execution_count == 0:
            locations.add((line_number, column_number))
    return sorted(locations)


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
    try:
        canonical_region_counts = _canonical_region_counts(data)
    except ValueError as error:
        canonical_region_counts = {}
        failures.append(str(error))

    totals = data.get("totals", {})
    metric_names = ["lines", "functions", "regions"]
    if arguments.require_branches:
        metric_names.append("branches")

    for metric_name in metric_names:
        metric = totals.get(metric_name)
        if not isinstance(metric, dict):
            failures.append(f"missing coverage metric: {metric_name}")
            continue
        raw_total, raw_covered = _metric_counts(metric)
        if metric_name == "regions" and canonical_region_counts:
            total = sum(value[0] for value in canonical_region_counts.values())
            covered = sum(value[1] for value in canonical_region_counts.values())
            if total != raw_total:
                failures.append(
                    "canonical source-region total disagrees with LLVM aggregate: "
                    f"canonical={total}, raw={raw_total}"
                )
            if covered != raw_covered:
                print(f"regions raw LLVM instances: {raw_covered}/{raw_total}")
        else:
            total, covered = raw_total, raw_covered
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
            raw_total, raw_covered = _metric_counts(metric)
            if metric_name == "regions" and filename in canonical_region_counts:
                total, covered = canonical_region_counts[filename]
            else:
                total, covered = raw_total, raw_covered
            if total != covered:
                incomplete_metrics.append(f"{metric_name}={covered}/{total}")
        if incomplete_metrics:
            print(f"incomplete file: {filename}: {', '.join(incomplete_metrics)}")
            missing_lines = _uncovered_lines(data, filename)
            if missing_lines:
                joined_lines = ", ".join(str(line_number) for line_number in missing_lines)
                print(f"uncovered lines: {filename}: {joined_lines}")
            segment_starts = _uncovered_segment_starts(file_record)
            if segment_starts:
                joined_locations = ", ".join(
                    f"{line_number}:{column_number}"
                    for line_number, column_number in segment_starts
                )
                print(f"uncovered segment starts: {filename}: {joined_locations}")

    if failures:
        print("; ".join(failures), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
