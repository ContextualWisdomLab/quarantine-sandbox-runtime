# PR source traversal relative-path invariant traceability

Last reviewed: 2026-09-13

## Problem and causal RED

Exact command-owner head `c70cc3622a9965d002d604c6bf6c61d034f84efd` ran native CI `34707886045`. Verify `103591175526` and hosted negative rootless/AppArmor `103591175499` were GREEN. Production and branch coverage generated immutable evidence and then failed only the repository-wide 100% admissions. Production artifact `10302217625` has digest `sha256:04ef81f1f77e9181e13164c8f3fc383483b459285d264aa574c5b1d3c6d7cb5b`; branch artifact `10301863269` has digest `sha256:0dd5fef1bbc21bd909cfc4b9be33c85a9f5e4320046ec71c9daf760a3673659e`.

The exact branch artifact measured 5045/5114 lines, 479/483 functions, 6772/6929 regions, and 710/718 branches. `src/pr_source_artifact.rs` alone measured 218/221 lines, 20/21 functions, 293/314 regions, and 32/32 branches. The missing source function was the `strip_prefix(root).map_err(...)` closure inside `collect_regular_files`.

That closure contradicted the implementation's own traversal invariant. Every `path` is a `DirEntry::path()` child of the directory passed to `read_dir`, and every recursive directory is reached from the same rooted traversal. The code comment already stated that prefix failure was unreachable, but the implementation still represented it as a production error branch.

## Decision

Carry the relative directory path as explicit traversal state. For each entry, derive the child-relative path with `relative_directory.join(entry.file_name())`; recurse with that derived relative directory. The root call begins with `Path::new("")`.

This keeps exact Unix filename bytes, including non-UTF-8 names, because `DirEntry::file_name()` returns `OsString` and the existing manifest continues to hash `OsStrExt::as_bytes()`. It preserves no-follow metadata inspection, unsupported-entry rejection, deterministic byte sorting, size limits, mutation detection, and staged-tree digest semantics.

Rejected alternatives:

- a synthetic fixture that tries to make a `read_dir` child escape the directory that produced it;
- `expect`, panic, or `unsafe` to silence the impossible branch;
- coverage exclusion or denominator manipulation;
- lossy string conversion or normalization of path identity.

## Evidence and acceptance

The one-shot repair workflow must run repository validation, rustfmt, focused source-staging regressions, the full locked workspace/all-target suite, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check`. It removes itself in the source commit. The bot-authored source descendant is not merge authority; a connector-authored ordinary descendant must reacquire native exact-head CI and immutable coverage evidence before the repair can be called GREEN.

Acceptance is source-coordinate closure of the unreachable `collect_regular_files` error function without weakening the 32/32 branch result, non-UTF-8 pathname preservation, symlink/unsupported-entry rejection, digest checks, or TOCTOU guards.

## Primary reference

Rust Project. (2026, September 4). *DirEntry in std::fs*. Rust standard library. https://doc.rust-lang.org/beta/std/fs/struct.DirEntry.html

Rust Project. (2026, September 1). *read_dir in std::fs*. Rust standard library. https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
