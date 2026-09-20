# Runtime gate release-pipe invariant traceability

## Decision record

PR #112 owns this command-runtime repair on top of canonical command owner PR #14. Application-service lifecycle and receipt authority remain owned by PR #21 / issue #113 and are not reimplemented here.

`RuntimeGatePodmanAdapter::release_command_gate` creates the local `podman attach --sig-proxy=false` control client with both stdin and stdout explicitly configured as `Stdio::piped()`. The prior implementation nevertheless carried separate `Option::None` branches after a successful `spawn()`. Those branches were not useful hostile-workload evidence: the call site itself requests both pipes, while the actionable release-control failures are attach spawn, release-token write/flush, acknowledgement EOF/contradiction/timeout, and detach/kill/reap failure.

The first simplification candidate, exact `6fa34e2ee54aea4bba5093e98508aae24a007bfd`, replaced the unreachable branches with `expect`. Native CI `34680019374` rejected that candidate in `tests/ddd_architecture.rs::production_source_has_no_panic_shortcuts`. That RED was correct: a structural invariant does not justify introducing a production panic shortcut in a hostile-workload boundary.

Exact `e0a537a68f9f1dd9694812f3f1b380c761240ad3` replaced the panic shortcut with a small provider-neutral `take_release_pipe` guard and a deterministic unit witness for both present and absent handles. Native CI `34680190502` then exposed only rustfmt layout drift. Formatter-only descendant `173b26b58fba070d3e9ee254ec619db2df221088` reacquired exact-head hosted verification.

The selected design keeps the call site fail closed without `unwrap`, `expect`, `panic!`, `unreachable!`, unsafe assumptions, retries, sleeps, coverage exclusions, or a fabricated `Child` state. The defensive contradiction is tested directly at the guard boundary. The real release client still uses the acquired exact 64-lowerhex container identifier, `--sig-proxy=false`, bounded acknowledgement, and explicit local attach cleanup.

## Alternatives

- **Panic on the configured-pipe invariant.** Rejected by the repository's production no-panic architecture gate; exact `6fa34e2...` is the causal RED.
- **Keep duplicate inline `Option::None` branches and manufacture an impossible spawned child.** Rejected because it would test a synthetic construction that this call site does not produce rather than a buyer-relevant runtime failure.
- **Use `unsafe` / unchecked extraction.** Rejected because the same invariant can be represented safely with no UB surface and no measurable hot-path benefit.
- **Remove fail-closed handling entirely.** Rejected. `Child` exposes captured handles as `Option`; the runtime therefore retains a provider-neutral defensive guard even though this configured call site expects both handles to be present.

## Exact-head evidence

Native CI `34680279795` on exact `173b26b58fba070d3e9ee254ec619db2df221088` produced the following evidence:

- `verify` `103517594594`: GREEN through exact checkout, dependency/repository/CI-contract validation, rustfmt, full workspace/all-target tests, Clippy with warnings denied, and rustdoc with warnings denied.
- hosted negative rootless/AppArmor `103517594579`: GREEN.
- production coverage `103517594576`: evidence generated and uploaded, then failed only the repository-wide 100% admission. Exact totals: 4830/4929 lines, 446/456 functions, 6445/6634 regions. Artifact ID `10292998041`, SHA-256 `2ae670e66dd08bcae17f418e3c5d7f9a3f8c1f697797bece708f8bef2cab43ef`.
- branch coverage `103517594614`: evidence generated and uploaded, then failed only the repository-wide 100% admission. Exact totals: 702/714 branches. Artifact ID `10293418543`, SHA-256 `78e72c1c8e7a8e3f4cb130e81ead292af8df25c28c7b1f41722d8ce1cb8b984e`.
- `src/infrastructure/podman_runtime_gate_binding.rs`: 14/18 branches, improving missed branches from the predecessor's 6 misses (16/22) to 4 without excluding source. Remaining uncovered release-control lines are concentrated in write/flush and detach/reap failure paths and must be covered through causal lifecycle seams rather than timing sleeps.
- dedicated positive SELinux acceptance remained runner-unassigned, so hosted negative AppArmor evidence is not promoted to positive-LSM acceptance.

The repository-wide deficit remains broader than this module. `src/infrastructure/podman.rs` is 193/196 branches and application-service coordinator is 19/24; the coordinator deficit belongs to PR #21, not this command-owner lane.

## Primary references

The Rust standard library documents `Command` as the process builder that configures child stdin/stdout/stderr and `Child` as exposing captured streams through `Option<ChildStdin>` / `Option<ChildStdout>`. It also documents that callers are responsible for waiting/reaping child processes, which is why the local attach-client cleanup remains explicit rather than relying on drop semantics.

The Rust Project Developers. (2026). *Command in std::process*. Rust standard library documentation. https://doc.rust-lang.org/std/process/struct.Command.html

The Rust Project Developers. (2026). *Child in std::process*. Rust standard library documentation. https://doc.rust-lang.org/std/process/struct.Child.html

The Rust Project Developers. (2026). *ChildStdin in std::process*. Rust standard library documentation. https://doc.rust-lang.org/std/process/struct.ChildStdin.html
