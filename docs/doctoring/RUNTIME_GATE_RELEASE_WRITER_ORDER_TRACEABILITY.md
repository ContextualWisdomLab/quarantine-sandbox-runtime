# Runtime-gate release-writer ordering traceability

Last reviewed: 2026-09-12

## Problem and invariant

The runtime-gate controller writes one release token to the exact acquired container through `podman attach --sig-proxy=false`. The controller must close that attach client's stdin before it begins local attach-client termination or reap work on a write/flush failure. Otherwise the cleanup path can interact with an attach process that is still waiting for EOF on the same control channel.

This is a lifecycle-ordering invariant, not a retry problem. The fix does not add sleeps, retries, `unsafe`, panic shortcuts, provider-specific assumptions, or workload/container termination authority.

## Causal RED

Exact predecessor `89e1993c5e4eae3c539900742bad4a1bb2434242` introduced deterministic release-client failure seams but moved write/flush-error cleanup into `write_release_payload_or_cleanup(&mut stdin, ..., &mut child)`. On that path the helper could enter `terminate_release_client` while the caller still owned live `stdin`; the caller's `drop(stdin)` was reached only after helper success.

Connector-authored staging exact `d3d7eb4fd8b1e38e9615430fa528bef8b2259524` installed a one-shot source-fix workflow. Run `34684960862`, job `103530240776`, first added a deterministic close witness: the scripted release client asserts that the writer's `Drop` witness is already true whenever cleanup invokes `try_wait`, `kill`, or `wait`. The focused `release_cleanup_starts_only_after_writer_is_closed` test then failed on the predecessor ordering with `release writer must be closed before cleanup`. The workflow required that exact failure before applying production code.

## Minimum causal repair

Ordinary descendant `182edadbd891842e68a322865fabe2f58a2db413` separates control-channel I/O from process cleanup:

- `write_release_payload` owns only write+flush and returns `io::Result<()>`;
- `release_command_gate` records the write result;
- on write/flush failure, `fail_after_release_writer_close` takes ownership of the writer, drops it, then invokes `fail_after_release_client_cleanup`;
- successful write still explicitly drops stdin before waiting for acknowledgement;
- detach failure keeps precedence over the original release-write failure when local attach-client cleanup itself fails.

The deterministic close-witness regression stays in production-module tests, so future refactors cannot make cleanup start with the control writer still live while leaving the existing process-script tests GREEN.

The one-shot workflow removed itself in the same descendant. It was not retained as a permanent mutation path.

## Exact verification

Before publishing `182edadb...`, source-fix run `34684960862` passed repository validation, CI-evidence parser tests, rustfmt, the focused close-order regression, full locked workspace/all-target/no-fail-fast tests, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check`.

That source-fix execution is causal repair evidence, not protected-release authority. The descendant still requires ordinary native exact-head CI/coverage, positive effective-LSM, qualifying independent review/security, real-runtime acceptance, protected integration, and immutable release evidence before merge or publication.

## Authority and alternatives

`quarantine-sandbox-runtime` owns this release control channel as part of the command-runtime infrastructure boundary. It does not move application-service lifecycle ownership away from canonical #21/#113, and it does not make generated sandbox names destructive authority.

Rejected alternatives:

- retrying write/flush or process cleanup, because retry does not establish EOF-before-cleanup ordering;
- sleeps or scheduler timing tests, because the ordering is directly observable with deterministic ownership/drop evidence;
- allowing cleanup to retain a borrowed writer, because that recreates the lifecycle coupling;
- suppressing the branch from coverage, because write/flush and detach failures are real runtime outcomes.

## Primary authority

Rust's current standard-library documentation states that dropping `ChildStdin` closes the underlying file handle and unblocks a child that was blocked on input. The `Child` documentation also states that child processes are not automatically waited on when their handles are dropped and that `wait` closes a retained stdin handle before waiting to avoid deadlock. Those process-lifecycle semantics support explicit writer closure before bounded attach-client cleanup rather than leaving the control writer alive while attempting process disposition.

### References

The Rust Project Developers. (2026). *Child in std::process*. Rust standard library 1.98.1. https://doc.rust-lang.org/std/process/struct.Child.html

The Rust Project Developers. (2026). *ChildStdin in std::process*. Rust standard library 1.98.0. https://doc.rust-lang.org/std/process/struct.ChildStdin.html
