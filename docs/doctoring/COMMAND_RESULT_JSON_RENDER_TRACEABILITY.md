# Command result JSON render traceability

Last reviewed: 2026-09-13

## Problem and exact evidence

PR #112 exact `957f978e8a0aae8eeb5c4c22c8bab54c50266b5b` produced immutable production coverage artifact `10311943614`. Its LLVM JSON records `src/main.rs` at 449/458 covered lines and 622/650 covered regions. The inline `serde_json::to_string_pretty(&result)` error arm is an owned production error boundary but had no deterministic witness; removing the arm, excluding it from measurement, or replacing it with an unwrap would weaken failure observability rather than repair the evidence gap.

## Decision

Extract only JSON presentation into private `report_result_json<T: serde::Serialize>`. Production behavior remains unchanged: successful rendering prints pretty JSON; serialization failure emits the existing warning; the sandboxed command exit code remains authoritative. The generic boundary allows a deterministic test serializer to return `serde::ser::Error`, while a normal JSON value proves the success arm. No provider/backend behavior, public schema, command execution, security policy, timeout, cleanup, or exit-code contract changes.

This is a testability seam for a real transport failure class, not synthetic application data and not a coverage exclusion. A moved head must reacquire exact CI and immutable coverage evidence; the `957f978...` artifact remains historical evidence only after this repair lands.

The first one-shot source-fix run `34744731032` failed before job materialization because the workflow embedded unindented Markdown inside a YAML literal block. No checkout or source mutation occurred. The workflow definition was repaired rather than bypassing that failure.

The corrected run `34744829753` then passed the focused witness, full workspace tests, and Clippy, but the deliberately stricter `--document-private-items` rustdoc check exposed two pre-existing application-service documentation links outside this command-owner repair. That owner-path defect is tracked as issue #115 for canonical #21/#113 repair. This one-shot therefore verifies the repository's current exact rustdoc gate and removes itself after publishing the command-owner fix; it does not suppress or close #115.

Run `34744935492` then passed the focused serialization witness, full `cargo test --locked --all-targets --no-fail-fast`, Clippy with `-D warnings`, and the repository's exact rustdoc gate before publishing ordinary descendant `4bf411592f0752fdc4aaeb155f85e50c99ed8f20` (`test(cli): cover result JSON render failure`). The purpose-limited workflow removed itself in that same descendant.

The resulting pull-request CI run `34744987313` had no jobs and concluded `action_required`. This is not a source/test regression: GitHub documents that a pull-request `synchronize` event caused by a workflow using the repository `GITHUB_TOKEN` creates workflow runs in an approval-required state to prevent recursive automation. A normal owner-authored traceability descendant therefore reacquires the ordinary PR CI gate without changing the tested runtime behavior. Future source-fix workflows must not treat a `GITHUB_TOKEN`-authored descendant as independently CI-qualified until ordinary exact-head CI has actually executed.

## Rejected alternatives

- Suppress or delete serialization failure handling: loses operator-visible transport failure evidence.
- `unwrap`/`expect`: converts a presentation error into a process panic.
- Coverage exclusion or denominator filtering: hides the owned production edge instead of exercising it.
- Backend fixtures solely to manufacture serialization failure: couples a presentation concern to the isolation adapter and violates the bounded responsibility of the CLI transport.
- Editing the application-service rustdoc from #112: violates the canonical #21/#113 single-writer boundary; issue #115 owns that repair.
- Treating an approval-required zero-job run as exact-head GREEN: confuses event authorization with code verification.

## Primary references

GitHub. (2026). *GITHUB_TOKEN*. GitHub Docs. https://docs.github.com/en/actions/concepts/security/github_token

Serde Project. (2026). *Serialize in serde*. https://docs.rs/serde/latest/serde/trait.Serialize.html

Serde JSON Project. (2026). *to_string_pretty in serde_json*. https://docs.rs/serde_json/latest/serde_json/fn.to_string_pretty.html
