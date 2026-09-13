from __future__ import annotations

import unittest

from scripts.check_coverage import (
    _canonical_region_counts,
    _uncovered_lines,
    _uncovered_segment_starts,
)


class CanonicalRegionCountTests(unittest.TestCase):
    """Treat repeated codegen records as one exact source region."""

    @staticmethod
    def _payload(second_execution_count: int) -> dict[str, object]:
        filename = "/workspace/target.rs"
        return {
            "files": [
                {
                    "filename": filename,
                    "summary": {"regions": {"count": 1, "covered": 0}},
                }
            ],
            "functions": [
                {
                    "filenames": [filename],
                    "regions": [[10, 3, 10, 8, 0, 0, 0, 0]],
                },
                {
                    "filenames": [filename],
                    "regions": [
                        [10, 3, 10, 8, second_execution_count, 0, 0, 0]
                    ],
                },
            ],
        }

    def test_any_executed_codegen_instance_covers_the_source_region(self) -> None:
        self.assertEqual(
            _canonical_region_counts(self._payload(second_execution_count=1)),
            {"/workspace/target.rs": (1, 1)},
        )

    def test_all_zero_codegen_instances_leave_the_source_region_uncovered(self) -> None:
        self.assertEqual(
            _canonical_region_counts(self._payload(second_execution_count=0)),
            {"/workspace/target.rs": (1, 0)},
        )

    def test_secondary_filename_id_is_attributed_to_its_source_file(self) -> None:
        target = "/workspace/target.rs"
        data = {
            "files": [
                {
                    "filename": target,
                    "summary": {"regions": {"count": 1, "covered": 0}},
                }
            ],
            "functions": [
                {
                    "filenames": ["/workspace/macro.rs", target],
                    "regions": [[7, 1, 7, 8, 4, 1, 0, 0]],
                }
            ],
        }
        self.assertEqual(_canonical_region_counts(data), {target: (1, 1)})

    def test_coordinate_count_mismatch_fails_closed(self) -> None:
        data = self._payload(second_execution_count=1)
        data["files"][0]["summary"]["regions"]["count"] = 2  # type: ignore[index]
        with self.assertRaisesRegex(ValueError, "canonical source-region count disagrees"):
            _canonical_region_counts(data)


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


if __name__ == "__main__":
    unittest.main()
