# Command Mount-Set Traceability

## Authority and problem

Canonical owner: `ContextualWisdomLab/quarantine-sandbox-runtime`, command-execution path in `RootlessPodmanAdapter::verify_command_isolation`.

Issue #32 covers a fail-open mismatch between the requested command profile and Podman's effective container mount set. Before the causal repair, a request without `source_artifact` could receive an unrelated effective host bind and still be accepted because command isolation inspected mounts only when a source artifact was present. A source-bearing request likewise located one `/workspace` mount but did not prove that it was the only effective mount.

## Reproduced RED

PR #14 exact predecessor `d6e2e8a11acd738cb1a4db4cb4e0234067dd445a`, native CI `34271419713`, verify job `102213674293` passed exact checkout, dependency-lock validation, repository policy, coverage-parser tests, and rustfmt, then failed exactly at `podman_command_execution_mount_set_red::command_without_source_rejects_an_unexpected_host_bind_and_cleans_up`.

The fake Podman inspection exposed an otherwise-valid command container plus one effective bind from host `/etc` to `/unexpected-host`. The request had no source artifact. Production returned `Ok(CommandExecutionResult)` instead of `IsolationVerificationFailed { control_name: "command_mount_set" }`. This is the causal RED for issue #32, not an inferred or queued result.

## Minimum causal repair

Commit `e5c6bfa843a537e3a2b330bd40ac21d5d285d481` adds one mount-cardinality invariant before existing source-mount property checks:

- no-source command: effective `Mounts` count must be zero;
- source-bearing command: effective `Mounts` count must be exactly one.

The existing source-artifact checks remain authoritative for that single mount's `/workspace` destination lookup, read-only state, exact runtime-staged source, bind type, and `noexec`/`nosuid`/`nodev` restrictions. This keeps `/tmp` resource/tmpfs evidence separate from host/source mount authority and rejects extra or duplicate effective mounts without broadening the accepted profile.

Rejected alternatives: silently ignoring extra mounts; basename/path rewriting; allowing arbitrary read-only host mounts; treating requested `--volume` argv as proof of effective state; or weakening the RED to accept backend drift.

## Security and standards basis

Podman documents that `--volume`/`--mount` can attach host bind mounts, named volumes, and other mount types to a container, with bind mounts directly exposing host files or directories. It also documents `ro`, `noexec`, `nosuid`, and `nodev` as mount restrictions. This means an unrequested effective mount is additional filesystem authority, not harmless metadata.

NIST SP 800-190 §4.5.5 requires containers to run with the minimum filesystem permissions required, says local host filesystems should be mounted only rarely, and states that sensitive host directories must not be mounted into containers. Exact allow-list attestation of the command profile therefore implements least filesystem authority at the runtime boundary.

## Remaining acceptance

This commit is only the minimum source repair after the executed RED. Issue #32 remains open until the unchanged exact head passes full tests and coverage, source-bearing duplicate/extra-mount acceptance is covered, issue #25 guarantees positive attestation before hostile payload release, real rootless-Podman negative/positive mount behavior and cleanup are proven, positive effective-LSM evidence is available, and protected integration/release gates pass.

## References

National Institute of Standards and Technology. (2017). *Application container security guide* (NIST Special Publication 800-190). https://doi.org/10.6028/NIST.SP.800-190

Podman. (2026). *podman-run — Run a command in a new container*. https://docs.podman.io/en/latest/markdown/podman-run.1.html
