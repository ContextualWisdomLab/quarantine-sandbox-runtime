# Command release-callback reachability traceability

Last reviewed: 2026-09-13

## Problem and ownership

The command adapter retained a no-op closure in `run_legacy_command_at_for_test` even though the shared execution function invoked a release callback only when runtime-gate binding arguments were present. The legacy path always supplied `None` for those binding arguments. LLVM coverage therefore reported an unexecuted anonymous function at the closure source coordinate even after all genuine command/runtime-gate branches were covered.

This is command-runtime ownership. It does not change application-service lifecycle/readiness semantics owned by PR #21/#113 and does not create a synthetic hostile input merely to satisfy a denominator.

## Causal RED

Exact `2c66fca09e01b7301e1722b58f802f91ddb71543`, native CI `34704870262`, produced branch artifact `10301233710` (`sha256:bc3ede1d5cf85d56eaea79b6256974c4722a2bd6ec6f60ceabb568d69a18bd39`) with repository totals 5044/5114 lines, 479/484 functions, 6772/6929 regions, and 710/718 branches.

Instantiation analysis showed the legacy closure at `src/infrastructure/podman.rs` was never executed. Source inspection proved why: `run_legacy_command_at_for_test` supplied no runtime-gate binding, and the shared function guarded callback invocation on the presence of runtime-gate binding args. No scheduler, backend, malformed payload, or adversarial workload can make that closure execute without changing the call contract itself.

## Decision

Rejected alternatives:

- adding a fake positive workload solely to try to execute the closure: impossible under the current guard and would manufacture evidence;
- excluding the closure or changing coverage configuration: would hide structural debt;
- weakening runtime-gate release checks: unrelated to the defect;
- duplicating the command lifecycle into separate legacy and gated functions: materially increases security-sensitive code surface.

Selected repair: represent release capability as `Option<F>`. The legacy debug path passes `None::<fn(&str) -> Result<(), CommandExecutionError>>`; the runtime-gated path passes `Some(release_gate)`; the shared function invokes the callback only through `if let Some(release_gate)`. This removes the anonymous no-op function and preserves the real gated callback unchanged.

Rust's reference specifies that each closure expression creates a unique anonymous closure type. The standard library documents `Option` as the language's ordinary representation for an optional value or optional function argument and recommends pattern matching to act only on `Some`. Those semantics match this boundary directly.

## Executed repair evidence

Connector-authored exact `5e3c4a527eeb3cdf7f0b193b688986fd572df19d` staged a self-removing one-shot source repair. Run `34707552403`, job `103590244387`, passed:

- single-writer exact-head verification;
- exact one-occurrence source replacement;
- repository validation;
- `cargo fmt --check`;
- focused legacy command resource-option coverage;
- focused runtime-gate command integration coverage;
- full locked workspace/all-target/no-fail-fast tests;
- Clippy with warnings denied;
- rustdoc with warnings denied;
- `git diff --check`.

The job published ordinary descendant `5a146ada3f3bfc5a325ce99dbea95faedd4a218f` and removed the temporary workflow in the same commit. Its automatically associated PR CI was `action_required`, so it is causal pre-publication evidence rather than native exact-head admission. Connector-authored no-tree-change descendant `f337419fffab80509fd9da941168c588a7e01039` reacquires native CI without changing the validated source tree.

## Remaining risk and acceptance

This repair does not prove positive SELinux/AppArmor confinement, real cgroup-v2 and `/tmp` enforcement, over-lease cleanup behavior, application-service lifecycle/readiness double-failure semantics, or release publication. It also does not transfer PR #21's 100% hosted application-service coverage into #112; that evidence must enter through ordinary canonical ancestry.

Before merge or release, require native exact-current-head verify/coverage, dedicated positive effective-LSM, qualifying independent review/security, owner-safe ancestry integration, #35/#43 real-runtime evidence, protected-head validation, immutable publication, SBOM/provenance, reproducibility, and rollback evidence.

## References

The Rust Project Developers. (2026). *Closure expressions*. The Rust Reference. https://doc.rust-lang.org/reference/expressions/closure-expr.html

The Rust Project Developers. (2026). *std::option*. Rust Standard Library 1.98.1. https://doc.rust-lang.org/std/option/
