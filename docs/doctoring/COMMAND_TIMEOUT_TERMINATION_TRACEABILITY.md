# Command timeout termination traceability

## Decision

A request lease expiry is an enforcement boundary, not merely an observation timestamp. `RootlessPodmanAdapter::wait_for_command` may report `timed_out=true` only after runtime-owned termination of the exact invocation-owned container has been successfully issued and the stopped/exited state has been observed. A failed timeout kill is infrastructure uncertainty and must fail closed before workload logs become trusted evidence.

## Current gap

Draft PR #14 currently bounds the first `podman wait` by `CommandExecutionRequest.resources.lease_seconds`. When that observer deadline expires, production invokes `podman kill` through `command_succeeded` but discards the Boolean result. It then performs another bounded `podman wait` and can return a successful `CommandExecutionResult` if the workload exits later on its own.

That behavior conflates two different facts:

- the observer deadline expired; and
- the runtime actually enforced termination at that deadline.

Only the first is currently proven. Issue #43 and `tests/podman_command_execution_timeout_kill_red.rs` isolate this gap without changing production.

## Authority

Podman Authors. (2026). *podman-kill — Kill the main process in one or more containers*. Podman states that `podman kill` sends SIGKILL by default to the selected container. https://docs.podman.io/en/latest/markdown/podman-kill.1.html

Podman Authors. (2026). *podman-wait — Wait on one or more containers to stop and print their exit codes*. `podman wait` waits for stopped/exited state and reports the container exit code; it does not establish that a preceding kill operation succeeded. https://docs.podman.io/en/latest/markdown/podman-wait.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

## Evidence chain

`CommandExecutionRequest.resources.lease_seconds`
→ bounded first `podman wait`
→ issue #43 causal RED (`15da2c53cea404b3fedef926ee29a1e5b31c20d9`)
→ failed `podman kill` must become `BackendCommandFailed { operation: "command_kill" }`
→ cleanup still attempted
→ no `podman logs` trust under uncertain termination
→ future real-Podman beyond-lease behavioral acceptance
→ integrated protected-head release evidence.

## Relationship to adjacent controls

- #25 prevents hostile payload execution before effective isolation proof.
- #35 binds configured Podman timeout and tmpfs state; configuration is not behavioral enforcement.
- #36 requires post-create lifecycle operations, including timeout kill, to target the acquired long container ID rather than a mutable name.
- #43 requires the termination operation itself to succeed before a timed-out result can become trusted evidence.

These controls compose. Repairing one does not subsume the others.

## Smallest causal GREEN after executed RED

After the RED executes for the intended ignored-kill-result cause, make timeout termination explicit and checked. Use the acquired container ID required by #36, require `podman kill` success, and only then perform the bounded post-kill wait. If kill fails, preserve cleanup attempts and return a typed backend error without reading logs as trusted workload evidence.

The first GREEN must not claim hard real-time precision. Release acceptance requires a real rootless-Podman workload that intentionally exceeds its lease, measured termination behavior, and leak-free cleanup on the exact candidate.