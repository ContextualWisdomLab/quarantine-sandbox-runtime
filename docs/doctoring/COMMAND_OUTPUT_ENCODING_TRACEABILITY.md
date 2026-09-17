# Command Output Encoding Traceability

## Decision scope

This record owns the command-execution evidence boundary for issue #34. `RootlessPodmanAdapter::run_command_at` receives retained `podman logs` stdout/stderr as bytes, while `CommandExecutionResult` schema `1.0.0` exposes both streams as Rust `String` / JSON text. The runtime therefore has to prove that retained bytes are valid UTF-8 before publishing them as text evidence.

This is an application-service supporting-context invariant implemented by the Podman infrastructure adapter. It does not authorize artifact verdicting, change the sandbox isolation policy, or widen the result schema into a binary transport.

## Causal RED

On exact predecessor `d99feb23f44868f6fcde08725cf2f9551b7f529e`, native CI `34274575395`, verify job `102224306639` passed exact checkout, dependency-lock validation, repository policy, coverage-parser tests, rustfmt, and the preceding command-runtime suites. It then failed exactly in `tests/podman_command_execution_output_encoding_red.rs`:

- `invalid_utf8_stdout_is_rejected_without_lossy_evidence` received `Ok(CommandExecutionResult)` from an otherwise-positive fake Podman that emitted invalid UTF-8 on stdout;
- `invalid_utf8_stderr_is_rejected_without_lossy_evidence` received `Ok(CommandExecutionResult)` from the corresponding stderr case;
- both fixtures require successful cleanup of the exact command container.

The production cause was `String::from_utf8_lossy(...)` in `RootlessPodmanAdapter::run_command_at`. Rust specifies that this conversion substitutes invalid UTF-8 sequences with U+FFFD, so the returned text can differ from the bytes observed from the workload. That mutation is incompatible with forensic/evidence semantics.

## Minimum causal repair

The repair lineage culminating in source candidate `118a025c8a4fe5747d04b8910c2e0958965aebc8` is intentionally small:

1. `a8fe4317e6a2e8ef77f28d4f3a80914ae4744267` adds `CommandExecutionError::InvalidOutputEncoding { stream }`, whose payload is only the stable `stdout`/`stderr` identifier and never workload bytes.
2. `87596f5713544091478f28f304456b288bbcd5d1` strengthens the RED to require that exact typed error while retaining the cleanup assertion.
3. `118a025c8a4fe5747d04b8910c2e0958965aebc8` replaces both lossy conversions with `String::from_utf8`. UTF-8 conversion occurs only after exact container cleanup has succeeded, so cleanup-failure precedence remains unchanged; valid UTF-8 continues through the existing `1.0.0` result contract unchanged.

Fresh exact-head CI for `118a025c8a4fe5747d04b8910c2e0958965aebc8` is `34279689168`. At the time this record was written its verify, coverage, branch-coverage, hosted negative rootless/AppArmor, and positive effective-LSM jobs had materialized but were still queued. The candidate is therefore not claimed GREEN by this document.

## Alternatives rejected

- Keep `String::from_utf8_lossy`: rejected because replacement-character insertion silently mutates evidence.
- Reject any output containing U+FFFD: rejected because a workload may legitimately emit the Unicode replacement character; text content cannot identify whether it was original or introduced by decoding.
- Truncate or sanitize invalid byte sequences: rejected because that creates another silent evidence transformation and conflates encoding validity with the existing bounded-output/truncation contract.
- Base64-encode arbitrary bytes inside the existing `stdout`/`stderr` string fields: rejected because it silently changes the meaning of released schema `1.0.0`. A binary-safe result requires a separately versioned contract with explicit encoding/digest metadata.
- Decode before cleanup and return immediately: rejected because an encoding defect must not suppress cleanup or hide a leaked sandbox. Cleanup failure remains the higher-priority security error.

## Security and buyer effect

The repair makes the text-only evidence contract fail closed rather than publishing altered workload output. Downstream review, malware-analysis, compiler/test, and forensic consumers can distinguish “valid retained text” from “this byte stream cannot be represented by the current text contract” without receiving potentially sensitive invalid bytes in an error message. It does not yet make runtime log storage complete: issue #31 still owns finite host-log storage plus real log-flood/incompleteness evidence.

## Release gates and follow-up

Before this delta can contribute to an immutable release, one unchanged exact head must pass the full Rust suite, Clippy/rustdoc, complete owned-production coverage and branch coverage, repository policy, qualifying review, security/dependency gates, hosted negative confinement, dedicated positive effective-LSM, and applicable real rootless-Podman E2E. Predecessor GREEN does not transfer to a moved head.

If buyers require arbitrary binary stdout/stderr, introduce a new versioned result contract that carries byte-safe payloads and explicit encoding/digest semantics. Do not reinterpret schema `1.0.0` in place.

## References

Bray, T. (2017). *The JavaScript Object Notation (JSON) Data Interchange Format* (RFC 8259). RFC Editor. https://doi.org/10.17487/RFC8259

Rust Project. (2026). *String in std::string (Rust 1.98.1): `String::from_utf8` and `String::from_utf8_lossy`*. https://doc.rust-lang.org/stable/std/string/struct.String.html
