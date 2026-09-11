# AppArmor admission-invariant traceability

Last reviewed: 2026-09-12

## Decision boundary

`RootlessPodmanAdapter` admits backend security evidence through `validate_backend_security` before any application-service or command container is created. Admission requires rootless execution, an enabled nonempty seccomp profile, and at least one enabled LSM signal: AppArmor or SELinux.

`effective_lsm_verified` then gives SELinux precedence. When `selinux_enabled` is true, the function returns directly from the SELinux label comparison. Reaching the AppArmor path therefore means SELinux is disabled. Combined with the already-admitted `apparmor_enabled || selinux_enabled` invariant, AppArmor must be enabled at that point.

The earlier implementation nevertheless guarded the AppArmor parser with a second `if apparmor_enabled` and ended with a separate `false`. That fallback described a state the public adapter cannot reach after successful backend admission. It was structural control-flow debt, not a hostile runtime outcome that needed a synthetic fixture.

The repair removes only that redundant admission check and terminal fallback. It keeps every effective AppArmor proof requirement: a nonempty runtime label, a parseable `<profile> (<mode>)` task context, `enforce` mode, a nonempty configured profile, rejection of `unconfined`, and equality between configured and live profiles. SELinux behavior is unchanged.

This change must not be cited as positive AppArmor confinement evidence. Real confinement still requires live process evidence through the adapter, and release acceptance still requires the dedicated positive-LSM lane on an eligible environment.

## Causal execution evidence

Exact `7c5f93d44877a4c4a87ddbaad9575f6ef42e629f` staged a temporary source-fix workflow, but push run `34638819190` failed before job creation. The workflow embedded an unescaped multiline Rust replacement inside a YAML block scalar; lines with less indentation escaped the `run: |` scalar, so GitHub could not construct any job. `jobs=[]` is workflow-definition failure evidence, not runner starvation and not a semantic test failure.

Exact `f0ab6795343924862ece4b57a38f4c7bf2412678` repaired the temporary workflow by carrying the old/new source fragments as base64 environment data. One-shot run `34642535866`, job `103405474936`, passed the single-writer exact-head check, applied the one-occurrence replacement, and passed repository validation, rustfmt, the focused `podman_command_lsm_shape_coverage` test, full workspace/all-target/no-fail-fast tests, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check` before publication.

The workflow then published ordinary descendant `738bc28863e25c50ecd759e8dd7455e85ec56027` (`fix(runtime): encode admitted AppArmor invariant`) and removed itself in the same commit. The production delta is limited to `src/infrastructure/podman.rs::effective_lsm_verified`; no coverage exclusion, ignored branch, weakened LSM predicate, provider assumption, or denominator configuration was introduced.

Because the descendant was pushed by `GITHUB_TOKEN`, its automatically associated CI run `34642660709` was `action_required` with zero jobs. That run is not exact-head GREEN. Connector-authored exact `3cfdc77de42da5488ba42c9352a69b3723d3c562` therefore reacquired native CI rather than transferring the one-shot result.

Native CI `34642752702` on exact `3cfdc77...` made verify `103406184230` GREEN through exact checkout, dependency/repository/CI-contract validation, rustfmt, full workspace/all-target/no-fail-fast tests, Clippy with warnings denied, and rustdoc with warnings denied. Hosted negative rootless/AppArmor `103406184554` was also GREEN. Coverage and branch coverage generated immutable evidence and failed only the repository-wide 100% admissions. Exact totals were 4802/4916 lines (97.68%), 442/453 functions (97.57%), 6405/6615 regions (96.83%), and 705/720 branches (97.92%). The branch artifact was `sha256:f34cc6a840e77190a3cabe6ce4ad9b6237105f75646aa9afe4307e3efdfc5f65`; the production coverage artifact was `sha256:ead603cf80e9db9f319c86bf831dd9da0107c0204f6f828be1c3635d5e8b606a`. File-level branch evidence narrowed `podman.rs` to 194/198, `podman_runtime_gate_binding.rs` to 16/22, and `application_service/coordinator.rs` to 19/24. Positive SELinux remained runner-unassigned.

That branch artifact exposed one further structural predicate in `effective_lsm_verified`: after `runtime_label = process.lsm_label.trim()` has succeeded and `runtime_label.rsplit_once(" (")` has returned `Some`, the extracted profile prefix cannot become empty after its own `trim()`. A prefix made only of whitespace would have been removed by the earlier whole-label trim, eliminating the required `" ("` delimiter before the split. The false outcome of `!runtime_profile.is_empty()` is therefore unreachable through this parser, not a missing hostile input class.

Exact `7c87eafdcef76a3b7862ec616746914984ce843b` staged the bounded repair. One-shot run `34643264616`, job `103407861712`, removed only that redundant predicate, removed its own workflow, and passed repository validation, rustfmt, the focused AppArmor shape regression, full workspace/all-target/no-fail-fast tests, Clippy with warnings denied, rustdoc with warnings denied, and `git diff --check`. It then published ordinary descendant `fcfcc2af3cd118c1f2270671bbc70251ef367a72` (`fix(runtime): remove unreachable empty AppArmor profile branch`). No live profile equality, enforce-mode, configured-profile, `unconfined`, SELinux, seccomp, capability, cleanup, or coverage-admission control was weakened.

Connector-authored exact `123a7e6c26d07dc2a2b4d96672c7d9da4d6693fc` reacquired native CI after that source repair. CI `34643438648` made verify `103408489205` and hosted negative rootless/AppArmor `103408489126` GREEN. Coverage and branch coverage generated immutable evidence and failed only the repository-wide 100% admissions. Exact totals were 4801/4915 lines (97.68%), 442/453 functions (97.57%), 6404/6614 regions (96.82%), and 704/718 branches (98.05%). The branch artifact was `sha256:f117e0731d0483d83fd27d1fecc3bfc7a837cd838b3839cee75b9845e088d5a7`; the production coverage artifact was `sha256:4a49de476c3a7a46b4c7ba6395edbd00f6dafbbd96a37e2cd5fec2d714a16cd7`. Positive SELinux job `103408488843` remained runner-unassigned.

The current branch artifact narrowed `podman.rs` to 193/196 branches, `podman_runtime_gate_binding.rs` to 16/22, and `application_service/coordinator.rs` to 19/24. The remaining Podman branch is the real post-probe readiness deadline path. Runtime-gate deficits are attach-pipe/write/terminate/reap failure outcomes; coordinator deficits include cleanup/registry double-failure precedence. They remain repair/test work rather than candidates for exclusion or impossible fixtures.

The product/technical gap ledger was also stale relative to this exact authority. Exact `89296c352dd4d8c5e94c0497e54ef899923457a9` staged a bounded documentation-only workflow. One-shot run `34644253281`, job `103411110993`, passed the single-writer exact-head check, prepended a 2026-09-12 current-authority supersession to `docs/product-technical-gap-baseline.md`, passed repository validation, rustfmt and `git diff --check`, verified the marker, removed its own workflow, and published ordinary descendant `8b725296ccfd3b6898ffdd4aaac668a1c10cb7c9`. The historical ledger was preserved byte-for-byte below the new supersession rather than replaced from a truncated connector view. This traceability descendant exists to reacquire native exact-head CI for that documentation repair; neither `123a7...` GREEN nor the one-shot GREEN transfers to the moved head.

## Reproducible acceptance

For the exact candidate intended for merge, require all of the following:

- `python3 scripts/validate_repository.py`;
- `cargo fmt --check`;
- `cargo test --locked --test podman_command_lsm_shape_coverage`;
- `cargo test --locked --workspace --all-targets --no-fail-fast`;
- `cargo clippy --locked --workspace --all-targets -- -D warnings`;
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`;
- repository-wide 100% owned-production statement/function/region/branch admission;
- hosted negative rootless/AppArmor evidence plus an actually executed positive effective-LSM lane;
- the separate issue #35/#43 real cgroup-v2, live `/tmp`, over-lease termination, and leak-free cleanup evidence;
- independent review/security and protected integrated release evidence.

## References

Linux Kernel documentation. (n.d.). *AppArmor*. Retrieved September 12, 2026, from https://www.kernel.org/doc/html/latest/admin-guide/LSM/apparmor.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
