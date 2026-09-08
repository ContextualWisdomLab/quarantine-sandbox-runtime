from __future__ import annotations

import unittest

from scripts.check_coverage import (
    _source_region_counts,
    _uncovered_lines,
    _uncovered_segment_starts,
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
                    "regions": [[10, 5, 10, 20, 7, 0, 0, 0]],
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


if __name__ == "__main__":
    unittest.main()
