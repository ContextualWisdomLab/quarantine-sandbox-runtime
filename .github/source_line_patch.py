from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


coverage_path = Path("scripts/check_coverage.py")
coverage = coverage_path.read_text(encoding="utf-8")
region_anchor = "def _source_region_counts(data: dict[str, Any]) -> tuple[int, int]:\n"
line_helpers = '''def _source_line_map_from_functions(
    data: dict[str, Any], production_filenames: set[str]
) -> dict[tuple[str, int], bool]:
    """Union physical source-line execution across LLVM function instantiations."""

    source_lines: dict[tuple[str, int], bool] = {}
    functions = data.get("functions")
    if not isinstance(functions, list):
        raise ValueError("coverage data has no function regions")

    for function in functions:
        if not isinstance(function, dict):
            raise ValueError("coverage function record is malformed")
        filenames = function.get("filenames")
        regions = function.get("regions")
        if not isinstance(filenames, list) or not isinstance(regions, list):
            raise ValueError("coverage function record is malformed")
        for region in regions:
            if not isinstance(region, list) or len(region) < 8:
                raise ValueError("coverage region record is malformed")
            file_id = int(region[5])
            if file_id < 0 or file_id >= len(filenames):
                raise ValueError("coverage region file id is out of range")
            filename = str(filenames[file_id])
            if filename not in production_filenames:
                continue
            executed = int(region[4]) > 0
            for line_number in range(int(region[0]), int(region[2]) + 1):
                key = (filename, line_number)
                source_lines[key] = source_lines.get(key, False) or executed
    return source_lines


def _source_line_map_from_segments(
    file_records: list[dict[str, Any]],
) -> dict[tuple[str, int], bool]:
    """Reconstruct physical source-line execution from LLVM file segments."""

    source_lines: dict[tuple[str, int], bool] = {}
    for file_record in file_records:
        filename = file_record.get("filename")
        segments = file_record.get("segments")
        if not isinstance(filename, str) or not isinstance(segments, list):
            raise ValueError("coverage file record has no line segments")

        line_regions: dict[int, list[tuple[bool, int]]] = {}
        for index, segment in enumerate(segments[:-1]):
            if not isinstance(segment, list) or len(segment) < 6:
                raise ValueError("coverage segment record is malformed")
            next_segment = segments[index + 1]
            if not isinstance(next_segment, list) or len(next_segment) < 2:
                raise ValueError("coverage segment record is malformed")

            line_start = int(segment[0])
            column_start = int(segment[1])
            execution_count = int(segment[2])
            has_count = bool(segment[3])
            is_gap_region = bool(segment[5])
            line_end = int(next_segment[0])
            column_end = int(next_segment[1])
            if not has_count or (line_end, column_end) <= (line_start, column_start):
                continue

            touched_lines = list(range(line_start, line_end))
            if line_end == line_start or column_end > 1:
                touched_lines.append(line_end)
            for line_number in touched_lines:
                line_regions.setdefault(line_number, []).append(
                    (is_gap_region, execution_count)
                )

        for line_number, regions in line_regions.items():
            non_gap_counts = [count for is_gap, count in regions if not is_gap]
            counts = non_gap_counts or [count for _is_gap, count in regions]
            source_lines[(filename, line_number)] = any(count > 0 for count in counts)
    return source_lines


def _source_line_counts(data: dict[str, Any]) -> tuple[int, int]:
    """Return physical source-line coverage after unioning codegen instantiations."""

    raw_file_records = data.get("files")
    if not isinstance(raw_file_records, list):
        raise ValueError("coverage data has no file records")

    file_records: list[dict[str, Any]] = []
    production_filenames: set[str] = set()
    for file_record in raw_file_records:
        if not isinstance(file_record, dict):
            raise ValueError("coverage file record is malformed")
        filename = file_record.get("filename")
        summary = file_record.get("summary")
        if not isinstance(filename, str) or not isinstance(summary, dict):
            raise ValueError("coverage file record is malformed")
        line_metric = summary.get("lines")
        if not isinstance(line_metric, dict):
            raise ValueError(f"coverage file has no line summary: {filename}")
        _metric_counts(line_metric)
        file_records.append(file_record)
        production_filenames.add(filename)

    function_lines = _source_line_map_from_functions(data, production_filenames)
    segment_lines = _source_line_map_from_segments(file_records)
    if function_lines.keys() != segment_lines.keys():
        raise ValueError(
            "source line denominator differs between function regions and file segments"
        )
    disagreements = [
        key
        for key, executed in function_lines.items()
        if segment_lines[key] != executed
    ]
    if disagreements:
        raise ValueError(
            "source line execution differs between function regions and file segments"
        )
    return len(function_lines), sum(function_lines.values())


'''
coverage = replace_once(
    coverage,
    region_anchor,
    line_helpers + region_anchor,
    "insert source-line helpers",
)
old_gate = '''        if metric_name == "regions":
            print(f"regions (LLVM raw): {covered}/{total}")
            try:
                total, covered = _source_region_counts(data)
            except ValueError as error:
                failures.append(str(error))
                continue
'''
new_gate = '''        if metric_name == "lines":
            print(f"lines (LLVM raw): {covered}/{total}")
            try:
                total, covered = _source_line_counts(data)
            except ValueError as error:
                failures.append(str(error))
                continue
        elif metric_name == "regions":
            print(f"regions (LLVM raw): {covered}/{total}")
            try:
                total, covered = _source_region_counts(data)
            except ValueError as error:
                failures.append(str(error))
                continue
'''
coverage = replace_once(coverage, old_gate, new_gate, "replace coverage admission")
coverage_path.write_text(coverage, encoding="utf-8")


test_path = Path("scripts/test_check_coverage.py")
tests = test_path.read_text(encoding="utf-8")
tests = replace_once(
    tests,
    '''from scripts.check_coverage import (
    _source_region_counts,
''',
    '''from scripts.check_coverage import (
    _source_line_counts,
    _source_region_counts,
''',
    "import source-line counter",
)
line_tests = '''class SourceLineCoverageTests(unittest.TestCase):
    """Measure one physical source line once across Rust codegen instantiations."""

    @staticmethod
    def _data(second_count: int = 7) -> dict[str, object]:
        return {
            "files": [
                {
                    "filename": "/workspace/src/runtime.rs",
                    "summary": {"lines": {"count": 2, "covered": 1}},
                    "segments": [
                        [10, 5, second_count, True, True, False],
                        [10, 20, 0, False, False, False],
                    ],
                }
            ],
            "functions": [
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[10, 5, 10, 20, 0, 0, 0, 0]],
                },
                {
                    "filenames": ["/workspace/src/runtime.rs"],
                    "regions": [[10, 5, 10, 20, second_count, 0, 0, 0, 0]],
                },
            ],
        }

    def test_mixed_instantiations_cover_one_physical_source_line(self) -> None:
        self.assertEqual(_source_line_counts(self._data()), (1, 1))

    def test_physical_line_is_uncovered_when_every_instance_is_zero(self) -> None:
        self.assertEqual(_source_line_counts(self._data(second_count=0)), (1, 0))

    def test_line_mapping_must_agree_between_regions_and_segments(self) -> None:
        data = self._data()
        data["files"][0]["segments"] = [
            [11, 5, 7, True, True, False],
            [11, 20, 0, False, False, False],
        ]
        with self.assertRaisesRegex(ValueError, "source line denominator"):
            _source_line_counts(data)


'''
tests = replace_once(
    tests,
    "class SourceRegionCoverageTests(unittest.TestCase):\n",
    line_tests + "class SourceRegionCoverageTests(unittest.TestCase):\n",
    "insert source-line tests",
)
tests = replace_once(
    tests,
    '"lines": {"count": 1, "covered": 1},\n',
    '"lines": {"count": 2, "covered": 1},\n',
    "set raw total lines",
)
tests = replace_once(
    tests,
    '"lines": {"count": 1, "covered": 1},\n',
    '"lines": {"count": 2, "covered": 1},\n',
    "set raw file lines",
)
tests = replace_once(
    tests,
    '                            "segments": [],\n',
    '''                            "segments": [
                                [10, 5, 7, True, True, False],
                                [10, 20, 0, False, False, False],
                            ],
''',
    "add line segments",
)
tests = replace_once(
    tests,
    '        self.assertIn("regions (LLVM raw): 0/1", stdout)\n',
    '''        self.assertIn("lines (LLVM raw): 1/2", stdout)
        self.assertIn("lines: 1/1", stdout)
        self.assertIn("regions (LLVM raw): 0/1", stdout)
''',
    "assert canonical line output",
)
test_path.write_text(tests, encoding="utf-8")


main_path = Path("src/main.rs")
main = main_path.read_text(encoding="utf-8")
old_machine = '''            let machine = match std::env::consts::ARCH {
                "x86_64" => 62_u16,
                "aarch64" => 183_u16,
                other => panic!("runtime-gate test fixture does not support architecture {other}"),
            };
'''
helper_anchor = '''        fn write_self_contained_gate(name: &str) -> (PathBuf, String) {
'''
helper = '''        #[cfg(target_arch = "x86_64")]
        fn test_elf_machine() -> u16 {
            62
        }

        #[cfg(target_arch = "aarch64")]
        fn test_elf_machine() -> u16 {
            183
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        fn test_elf_machine() -> u16 {
            panic!(
                "runtime-gate test fixture does not support architecture {}",
                std::env::consts::ARCH
            )
        }

'''
main = replace_once(main, helper_anchor, helper + helper_anchor, "insert architecture helper")
main = replace_once(
    main,
    old_machine,
    "            let machine = test_elf_machine();\n",
    "replace runtime architecture match",
)
main_path.write_text(main, encoding="utf-8")


doctoring = '''# Source-line Coverage Union Traceability

Last reviewed: 2026-09-13

## Problem and causal evidence

Exact predecessor `d82fa32a5fd79e30a2b0fb6e986e4701c5c137e8` produced branch artifact `10314340212` (`sha256:76487350507dcddd4a8e84e323cf37bdec9c22dd073d969dbe9eeff09a84189a`). LLVM reported 5260/5298 raw lines. Source-coordinate inspection of that immutable artifact showed that `src/main.rs` had 465/476 raw lines while source-union diagnostics isolated only two genuinely uncovered physical lines: the alternate-architecture and unsupported-architecture arms of the test-only ELF fixture. `src/main.rs` remained unchanged through causal predecessor `b760ccf4f01dc2735e0326b0a343e03c74a06a8a`.

The same artifact exposes the upstream Rust/LLVM multi-instantiation accounting problem already handled for source regions: one physical source location may execute in one monomorphization while another zero-count instantiation remains in the raw summary. LLVM defines line coverage by whether a code line executed at least once. Rust issue rust-lang/rust#137524 documents a reproducible multi-instantiation case where raw totals retain an uncovered line although another instantiation executes the same source and exported JSON cannot identify a corresponding zero-count physical region.

## Decision

Admission preserves and prints LLVM raw line totals, then separately unions physical source-line execution across function instantiations. It independently reconstructs the same source-line map from file-level coverage segments and fails closed if denominator or execution truth disagrees. No line, file, or metric is excluded and no denominator is sampled or manually reduced.

The test-only runtime-gate ELF fixture no longer models mutually exclusive host architectures as a runtime `match`. Architecture selection is compile-time `cfg`, so each supported target contains only the machine value that can execute. Unsupported targets still fail explicitly if this test helper is compiled. Production runtime behavior, public contracts, and supported x86_64/aarch64 machine values are unchanged.

## Validation contract

The repair must pass source-line regression tests for mixed and all-zero instantiations plus mapping disagreement, rustfmt, the full locked workspace/all-target test suite, Clippy with warnings denied, rustdoc with warnings denied, and branch coverage generation. Exact generated evidence must show no genuinely uncovered physical source line in `src/main.rs`; remaining source-line and branch deficits must stay attributable to real application-service owner paths rather than codegen-instance artifacts.

## References

LLVM Project. (2026). *Source-based code coverage*. Clang documentation. https://clang.llvm.org/docs/SourceBasedCodeCoverage.html

LLVM Project. (2026). *llvm-cov - emit coverage information*. LLVM documentation. https://llvm.org/docs/CommandGuide/llvm-cov.html

Rust Project. (2025). *Test Coverage: cannot find uncovered lines and regions* (rust-lang/rust#137524). GitHub. https://github.com/rust-lang/rust/issues/137524
'''
Path("docs/doctoring/COVERAGE_SOURCE_LINE_UNION_TRACEABILITY.md").write_text(
    doctoring, encoding="utf-8"
)
