# Command argv / image ENTRYPOINT traceability

## Decision under review

`CommandExecutionRequest.command` is a direct argv contract. The runtime must execute that exact validated argv without a shell and without silently prepending an OCI image-defined `ENTRYPOINT`.

`RootlessPodmanAdapter::run_command_at` currently constructs `podman create ... <image_reference> <request.command...>`. Podman treats the command following the image as the container command/arguments. When the image already defines an `ENTRYPOINT`, that entrypoint remains the executable unless `--entrypoint` overrides it. The current backend can therefore execute `image-entrypoint <requested argv...>` while returning evidence under the requested command contract.

Issue #38 owns this gap. Test-bearing commit `387bee4ecf61de9be1915f78c2cd43b73d7379cb` adds `tests/podman_command_execution_entrypoint_red.rs`. Its otherwise-positive fake-Podman path requires the complete requested argv, including an argument containing spaces, to be represented as one exact JSON-array entrypoint override and forbids appending the same argv after the immutable image reference.

## Evidence chain

- Contract authority: `CommandExecutionRequest.command` documents direct argv and no shell.
- Architecture authority: `docs/ARCHITECTURE.md` documents the runtime-to-Podman boundary as direct argv with no shell.
- Transport decision: ADR-0008 documents direct argv after `--` and rejects a shell-string parser.
- Current backend intent: `run_command_at` passes the digest-pinned image and then appends each requested argument after it.
- Podman semantics: `podman create --entrypoint` overrides the image's default ENTRYPOINT; without that override, an existing image ENTRYPOINT remains the executable and the supplied command is passed to it as arguments/options.
- Hostile RED: `tests/podman_command_execution_entrypoint_red.rs` at `387bee4ecf61de9be1915f78c2cd43b73d7379cb`.
- Smallest causal GREEN after executed RED: represent the complete validated request argv as Podman's JSON-array `--entrypoint` value, or an equivalent exact OCI argv primitive, without a second command layer after the image.
- Separate controls: #25 owns pre-attestation payload release; #37 owns applied immutable image digest; #33/#36 own lifecycle cleanup identity. Exact argv semantics do not substitute for those controls.

## Alternatives

Leaving the image ENTRYPOINT in place is rejected because it changes the executable that owns PID 1 and makes returned command evidence ambiguous. Joining argv into a shell string is rejected because it changes argument boundaries and would introduce shell parsing authority that the public contract explicitly excludes. Treating the image ENTRYPOINT as a consumer responsibility is rejected because the runtime itself claims direct-argv semantics and owns the backend translation.

## Verification rule

The test-only launch-intent RED is not sufficient release evidence. After the causal RED executes and the smallest repair is added, real rootless-Podman E2E should use a digest-pinned fixture image with a non-empty ENTRYPOINT and prove the requested executable/arguments are the effective OCI process without shell mediation. If Podman inspect exposes stable `Config.Entrypoint`/`Config.Cmd` evidence for the supported version set, bind those fields as backend-applied evidence before hostile payload release; otherwise use an equivalent runtime-owned execution proof.

## References

Podman Authors. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
