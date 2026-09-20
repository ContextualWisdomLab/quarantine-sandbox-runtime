# Command-wait output-limit traceability

Last reviewed: 2026-09-13

## Decision

`RootlessPodmanAdapter::wait_for_command` must not represent stdout/stderr truncation as a second decision after `BoundedCompletion::Exited` has already been selected. The owned `BoundedCommandRunner` contract classifies any observed stdout/stderr overflow as `BoundedCompletion::OutputLimit`; `Exited(status)` is therefore reachable only when no output overflow was observed.

The command owner keeps one fail-closed mapping for output overflow: `BoundedCompletion::OutputLimit` becomes `ApplicationServiceError::BackendOutputLimitExceeded { operation: "command_wait" }`. The former `stdout_truncated || stderr_truncated` guard inside the `Exited` arm duplicated an impossible state rather than protecting a distinct runtime outcome.

## Causal RED

Exact predecessor `0ab35c399a14aed9dc04e575826fc874ba82afcf` ran native CI `34729167091`. Hosted `verify` and negative rootless/AppArmor completed successfully. Branch artifact `10308302725` has digest `sha256:7afd53f86799a08ccb72fc05d6fbcaf5eca3f21eb2e8b4b121e44c7a88a3b4aa` and measured:

- 5220/5262 production lines;
- 503/503 production functions;
- 6999/7126 production regions;
- 720/730 production branches.

Source-coordinate aggregation placed `src/infrastructure/podman.rs` at 189/194 branches. Both predicates in the predecessor `wait_outcome.stdout_truncated || wait_outcome.stderr_truncated` expression had zero true executions while normal `Exited` outcomes were exercised. This is consistent with the upstream completion classifier: overflow is converted to `OutputLimit` before `Exited` can be constructed.

The same artifact isolated the remaining genuine branch gaps outside this command-owner invariant. The post-probe readiness deadline in `wait_for_readiness` and the application-service `cleanup_expired` backend-termination plus registry-finalization double-failure precedence remain application-service-owned and must flow through #21/#113 rather than being reproduced in the command lane.

## Repair and validation

The first source-fix workflow publication had invalid YAML block indentation and materialized no job (`34729859898`). That workflow definition was repaired without touching production code at `edddff29380c1941cfbf6e7b8fbe7beb5da73f9a`.

One-shot run `34729886700` then:

1. verified the exact predecessor source invariant and causal coverage RED;
2. removed only the structurally impossible `Exited` truncation gate from `src/infrastructure/podman.rs`;
3. passed `cargo fmt --all -- --check`;
4. passed `cargo test --locked --workspace --all-targets --no-fail-fast`;
5. passed `cargo clippy --locked --workspace --all-targets -- -D warnings`;
6. passed `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`;
7. passed `git diff --check`;
8. published `a6dda0d9aeed837caf865e2965c13cc374c65d67` and removed its own source-fix workflow.

`docs/product-technical-gap-baseline.md` was then superseded at `b6619fe89f83dda0348c9f3a3e580d4feb7bf406`; its purpose-limited updater removed itself in the same commit. This traceability file is an ordinary connector-authored descendant so native CI can evaluate a non-workflow final head. No predecessor GREEN status is transferred to that descendant.

## Invariants preserved

- Output overflow remains fail closed as `BackendOutputLimitExceeded`.
- Timeout remains a distinct `BoundedCompletion::TimedOut` path.
- Successful exit-code parsing occurs only for `BoundedCompletion::Exited(status)` with `status.success()`.
- Non-success exit status remains `BackendCommandFailed`.
- No coverage exclusion, ignore attribute, denominator override, retry, sleep, or synthetic impossible input was introduced.
- Application-service lifecycle/readiness truth remains in its canonical owner rather than being copied into the command owner.

## Release impact

This repair removes an impossible command-owner decision from the production denominator; it does not establish release readiness by itself. Fresh exact-head native coverage, dedicated positive effective-LSM evidence, stable application-service ancestry, qualifying independent review/security, #35/#43 real-runtime acceptance, protected-head integration, and immutable release/SBOM/provenance/reproducibility/rollback evidence remain required.
