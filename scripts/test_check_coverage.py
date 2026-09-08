from __future__ import annotations

import io
import json
import pathlib
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

from scripts.check_coverage import (
    _source_region_counts,
    _uncovered_lines,
    _uncovered_segment_starts,
    main,
)


class UncoveredLineAttributionTests(unittest.TestCase):
    """Keep LLVM region attribution aligned with each region's source file id."""

    def test_target_file_can_be_a_non_primary_function_filename(self) -> None:
        data = {
            "functions": [
                {
                    "filenames": ["/workspace/macro.rs", "/workspace/target.rs"],
                    "regions": [
                        [7, 1, 7, 8, 0, 1, 0, 0],
                        [8, 1, 8, 8, 1, 1, 0, 0],
                    ],
                }
            ]
        }

        self.assertEqual(_uncovered_lines(data, "/workspace/target.rs"), [7])

    def test_regions_from_other_file_ids_do_not_pollute_target_lines(self) -> None:
        data = {
            "functions": [
                {
                    "filenames": ["/workspace/target.rs", "/workspace/expanded.rs"],
                    "regions": [
                        [3, 1, 3, 8, 1, 0, 0, 0],
                        [99, 1, 99, 8, 0, 1, 0, 0],
                    ],
                }
            ]
        }

        self.assertEqual(_uncovered_lines(data, "/workspace/target.rs"), [])

    def test_file_segments_locate_zero_count_counted_region_starts(self) -> None:
        file_record = {
            "segments": [
                [10, 1, 4, True, True, False],
                [11, 5, 0, True, True, False],
                [11, 9, 0, False, False, False],
                [12, 1, 0, True, True, True],
                [13, 2, 0, True, True, False],
                [14, 1, 2, True, True, False],
            ]
        }

        self.assertEqual(_uncovered_segment_starts(file_record), [(11, 5), (13, 2)])


class SourceRegionCoverageTests(unittest.TestCase):
    """Measure source regions once even when LLVM exports multiple instantiations."""

    @staticmethod
    def _file_record(region_count: int) -> dict[str, object]:
        return {
            "filename": "/workspace/src/runtime.rs",
            "summary": {"regions": {"count": region_count, "covered": region_count}},
        }

    def test_mixed_instantiations_cover_one_shared_source_region(self) -> None:
        data = {
            "files": [self._file_record(1)],
            "functions": [
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[10, 5, 10, 20, 0, 0, 0, 0]],
                },
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[10, 5, 10, 20, 7, 0, 0, 0, 0]],
                },
            ],
        }

        self.assertEqual(_source_region_counts(data), (1, 1))

    def test_source_region_is_uncovered_when_every_instance_is_zero(self) -> None:
        data = {
            "files": [
                {
                    "filename": "/workspace/src/runtime.rs",
                    "summary": {"regions": {"count": 1, "covered": 0}},
                }
            ],
            "functions": [
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[20, 3, 20, 18, 0, 0, 0, 0]],
                },
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[20, 3, 20, 18, 0, 0, 0, 0]],
                },
            ],
        }

        self.assertEqual(_source_region_counts(data), (1, 0))

    def test_region_denominator_must_match_production_file_summaries(self) -> None:
        data = {
            "files": [self._file_record(2)],
            "functions": [
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[30, 1, 30, 9, 1, 0, 0, 0]],
                }
            ],
        }

        with self.assertRaisesRegex(ValueError, "source region denominator"):
            _source_region_counts(data)


class CoverageAdmissionTests(unittest.TestCase):
    """Gate source-region coverage without hiding LLVM's raw summary."""

    @staticmethod
    def _payload(file_region_count: int = 1) -> dict[str, object]:
        return {
            "data": [
                {
                    "totals": {
                        "lines": {"count": 1, "covered": 1},
                        "functions": {"count": 1, "covered": 1},
                        "regions": {"count": 1, "covered": 0},
                    },
                    "files": [
                        {
                            "filename": "/workspace/src/runtime.rs",
                            "summary": {
                                "lines": {"count": 1, "covered": 1},
                                "functions": {"count": 1, "covered": 1},
                                "regions": {
                                    "count": file_region_count,
                                    "covered": 0,
                                },
                            },
                            "segments": [],
                        }
                    ],
                    "functions": [
                        {
                            "filenames": ["/workspace/src/runtime.rs"],
                            "regions": [[10, 5, 10, 20, 0, 0, 0, 0]],
                        },
                        {
                            "filenames": ["/workspace/src/runtime.rs"],
                            "regions": [[10, 5, 10, 20, 7, 0, 0, 0, 0]],
                        },
                    ],
                }
            ]
        }

    def _run_main(self, payload: dict[str, object]) -> tuple[int, str, str]:
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "coverage.json"
            path.write_text(json.dumps(payload), encoding="utf-8")
            stdout = io.StringIO()
            stderr = io.StringIO()
            with (
                patch("sys.argv", ["check_coverage.py", str(path)]),
                redirect_stdout(stdout),
                redirect_stderr(stderr),
            ):
                result = main()
        return result, stdout.getvalue(), stderr.getvalue()

    def test_admission_uses_source_region_union_and_reports_raw_llvm_summary(self) -> None:
        result, stdout, stderr = self._run_main(self._payload())

        self.assertEqual(result, 0)
        self.assertEqual(stderr, "")
        self.assertIn("regions (LLVM raw): 0/1", stdout)
        self.assertIn("regions: 1/1", stdout)

    def test_admission_fails_closed_on_source_region_denominator_mismatch(self) -> None:
        result, _stdout, stderr = self._run_main(self._payload(file_region_count=2))

        self.assertEqual(result, 1)
        self.assertIn("source region denominator", stderr)


if __name__ == "__main__":
    unittest.main()
