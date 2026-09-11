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

Because the descendant was pushed by `GITHUB_TOKEN`, its automatically associated CI run `34642660709` was `action_required` with zero jobs. That run is not exact-head GREEN. This traceability commit intentionally moves the head again under the repository owner so native CI can execute on the resulting exact candidate; predecessor or one-shot GREEN must not be transferred to that moved head.

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
