# Git pathname byte traceability

## Scope

This note covers the exact-revision PR-source staging boundary in `src/pr_source_artifact.rs` and the focused RED in `tests/pr_source_artifact_non_utf8_path_red.rs` (issue #29; test-bearing commit `2733bd46d054ff92861d5383fcbf68fd151e77ef`). It is separate from issue #26, which addresses lossy reinterpretation of a literal Linux backslash byte as `/`.

## Current finding

`collect_regular_files` converts each relative path through `Path::to_str()` before the path is sorted, hashed, and copied. On Unix, `Path`/`OsStr` can represent pathname bytes that are not valid UTF-8. Git likewise does not require core pathnames to be valid UTF-8: core path identity is byte-oriented and commands expose NUL-delimited/verbatim pathname modes for precisely this reason.

The current implementation therefore rejects a materialized Git/Linux tree solely because one otherwise-valid pathname is not Unicode. That is an interoperability and artifact-identity gap for a runtime that claims to stage an exact source tree.

The test-bearing RED constructs one regular file whose relative filename contains byte `0xff`, computes the canonical manifest digest over the exact raw pathname bytes and file bytes, and requires staging to preserve that path one-to-one. Production is intentionally unchanged until the RED executes for the intended `Path::to_str()` failure.

## Decision boundary

The causal implementation should keep canonical pathname identity in `OsStr`/raw Unix bytes through collection, ordering, digesting, and destination joining. Human-facing diagnostics may render an escaped or lossy representation, but that representation must not feed back into canonical hashing or destination identity.

This does not define a cross-platform normalization scheme. `/` remains the Linux pathname component separator; literal backslash remains an ordinary filename byte as covered by #26. A future Windows/macOS transport requires a separate versioned representation and compatibility decision rather than silent normalization.

## Evidence links

- Owner issue: #29 `P1 artifact integrity: preserve non-UTF-8 Git pathname bytes during source staging`.
- RED: `tests/pr_source_artifact_non_utf8_path_red.rs` at `2733bd46d054ff92861d5383fcbf68fd151e77ef`.
- Production locus: `src/pr_source_artifact.rs::collect_regular_files` (`Path::to_str()`).
- Regression dependency: issue #26's `a\\b` versus `a/b` collision coverage must remain GREEN after #29 is repaired.

## References

Git Project. (n.d.). *git-config documentation*. https://git-scm.com/docs/git-config

Git Project. (n.d.). *git-show documentation*. https://git-scm.com/docs/git-show

Linux man-pages project. (n.d.). *pathname(7)—How pathnames are encoded and interpreted*. https://man7.org/linux/man-pages/man7/pathname.7.html
