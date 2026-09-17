# Command argv / image ENTRYPOINT traceability

## Decision

`CommandExecutionRequest.command` is a direct argv contract. The runtime must execute that exact validated argv without a shell and without silently prepending an OCI image-defined `ENTRYPOINT`.

The pre-repair `RootlessPodmanAdapter::run_command_at` constructed `podman create ... <image_reference> <request.command...>`. Podman treats the command following the image as the container command/arguments. When the image already defines an `ENTRYPOINT`, that entrypoint remains the executable unless `--entrypoint` overrides it. The backend could therefore execute `image-entrypoint <requested argv...>` while returning evidence under the requested command contract.

Issue #38 owns this gap. Test-bearing commit `387bee4ecf61de9be1915f78c2cd43b73d7379cb` adds `tests/podman_command_execution_entrypoint_red.rs`. Its otherwise-positive fake-Podman path requires the complete requested argv, including an argument containing spaces, to be represented as one exact JSON-array entrypoint override and forbids appending the same argv after the immutable image reference.

## Executed causal RED and minimum repair

On canonical owner PR #14 exact `4a5369de53e0b4c3b2ba446a134b5254098e7aae`, the re-run of native CI `34026602902` reached coverage job `102139910936` after all five `podman_command_execution_cleanup_red` cases passed. It then failed exactly at `podman_command_execution_entrypoint_red::requested_command_is_encoded_as_exact_entrypoint_argv`. The observed create argv contained the digest-pinned image followed by `/usr/bin/tool`, `argument with spaces`, and `--flag=value`, with no `--entrypoint`. This is canonical owner-path causal RED, not borrowed descendant evidence.

Commit `daf4d07820fbfe3abcefc39f0e8ce2443138e385` is the minimum production repair. It converts the already-validated `request.command` into one compact JSON array through `serde_json::Value`, supplies that value as `--entrypoint=<json-array>` before the image operand, and removes the duplicate command layer after the image. Digest pinning, source staging/workdir, network denial, cidfile ownership, isolation attestation, resource/time/output bounds, and cleanup behavior are unchanged. Application-service ENTRYPOINT issue #47 remains a separate owner path.

## Evidence chain

- Contract authority: `CommandExecutionRequest.command` documents direct argv and no shell.
- Architecture authority: `docs/ARCHITECTURE.md` documents the runtime-to-Podman boundary as direct argv with no shell.
- Transport decision: ADR-0008 documents direct argv after `--` and rejects a shell-string parser.
- Causal owner RED: PR #14 exact `4a5369de53e0b4c3b2ba446a134b5254098e7aae`, native CI `34026602902`, coverage attempt job `102139910936`.
- Hostile regression: `tests/podman_command_execution_entrypoint_red.rs`, introduced at `387bee4ecf61de9be1915f78c2cd43b73d7379cb`.
- Minimum candidate: `daf4d07820fbfe3abcefc39f0e8ce2443138e385` in `src/infrastructure/podman.rs`.
- Podman semantics: `podman create --entrypoint` overrides the image's default ENTRYPOINT; multi-option commands are represented as a JSON string/array.
- Separate controls: #25 owns pre-attestation payload release; #37 owns applied immutable image digest; #33/#36 own lifecycle cleanup identity. Exact argv semantics do not substitute for those controls.

## Alternatives

Leaving the image ENTRYPOINT in place is rejected because it changes the executable that owns PID 1 and makes returned command evidence ambiguous. Joining argv into a shell string is rejected because it changes argument boundaries and would introduce shell parsing authority that the public contract explicitly excludes. Treating the image ENTRYPOINT as a consumer responsibility is rejected because the runtime itself claims direct-argv semantics and owns the backend translation. Appending the same argv after the image while also setting `--entrypoint` is rejected because it creates a second command layer and changes the exact argv contract.

## Verification rule

The launch-intent RED and source repair are not sufficient release evidence. The unchanged repaired exact head must pass repository validation, rustfmt, focused/full Rust tests, Clippy/rustdoc, complete owned-production coverage and the repository's review/security gates. Real rootless-Podman E2E must use a digest-pinned fixture image with a non-empty ENTRYPOINT and prove the requested executable/arguments are the effective OCI process without shell mediation. If Podman inspect exposes stable `Config.Entrypoint`/`Config.Cmd` evidence for the supported version set, bind those fields as backend-applied evidence before hostile payload release; otherwise use an equivalent runtime-owned execution proof.

## References

Podman Authors. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
