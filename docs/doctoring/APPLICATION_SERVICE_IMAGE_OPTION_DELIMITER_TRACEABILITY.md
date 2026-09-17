# Application-service image option-delimiter traceability

## Current regression and bounded-context ownership

Canonical application-service/network parent #127 exact `1a3cd0427bbf99e41c1d8bd4089747561237071f` validates a consumer-supplied immutable image reference, then places that string directly after Podman `create` options and before consumer command argv without an explicit `--` terminator. Draft successor #133 repairs that lost boundary on the current #127 ancestry rather than transplanting an older owner tree.

The request validator accepts a repository component beginning with `-` and containing `=` when the overall value ends in a valid lower-case `@sha256:<64-hex>` digest. A Podman-shaped token such as `--annotation=qsr.boundary=value@sha256:<digest>` therefore reaches the infrastructure plan as domain-valid image data.

This is not a new control. It is a succession regression of reopened issue #90 / historical PR #91. `application_service` owns immutable workload intent; `infrastructure::podman` owns translation into Podman's CLI grammar. The invariant is that the digest-validated workload identity remains Podman's image positional operand and cannot be reinterpreted as provider control syntax.

## Historical causal evidence

PR #91 supplied the original focused application-service witness. Native CI `34202917157`, coverage job `101985631210`, executed `tests/podman_option_terminator_red.rs` and observed the policy-SHA label immediately before a domain-valid option-shaped image instead of literal `--`.

Minimum production commit `1b1026a92b7d9ef23a709564c1e22aa0421179ff` inserted exactly one literal `--` immediately before `request.image_reference` while preserving the image and following command argv entry-for-entry.

Canonical command-runtime #14 later succeeded the application-service delimiter, the focused regression and refreshed `PODMAN_OPTION_TERMINATOR_TRACEABILITY.md`, and added independent runtime-gate/command-path coverage. Exact #14 `6df79290321f697b2a23c9cf246575a4d13c5740`, CI `34489161644`, had verify `102911078197` GREEN through exact checkout, repository policy, rustfmt, full locked workspace/all-target tests, Clippy and rustdoc; hosted negative rootless/AppArmor `102911078027` was also GREEN.

Issue #90 and PR #91 were therefore legitimately completed at that point. Fresh inspection of #127 showed the verified delimiter had subsequently fallen out of the canonical application-service/network ancestry, so #90 was reopened.

## Parser evidence

Podman's current `cmd/podman/containers/create.go` declares `create [options] IMAGE [COMMAND [ARG...]]` and configures pflag with `SetInterspersed(false)`. pflag treats `--` as the explicit flag-parsing terminator. A leading-dash token presented where the image should begin is still eligible for option parsing until that boundary is terminated.

The request also contains consumer-controlled command argv. If the accepted image token is consumed as a Podman option, a following command token can become the first positional `IMAGE`. The SHA-256 identity validated by the runtime can then cease to describe the workload operand presented to Podman.

## Current-owner regression replay and repair

Issue #132 / Draft #133 is the current regression-instance lane. Test-only `088fd10e794b5c02a13ed45e082d509dc77ccfb9` adds `tests/podman_application_service_option_delimiter_red.rs` on exact #127 parent `1a3cd0427bbf99e41c1d8bd4089747561237071f`.

The witness:

- builds an otherwise-valid request with `--annotation=...@sha256:<digest>` as the image reference;
- independently proves request validation succeeds before inspecting backend argv;
- supplies a following image-shaped consumer command token so positional displacement is explicit;
- requires exactly one literal `--` immediately before the exact validated image reference;
- requires command argv after the image to remain entry-preserving.

Historical #90/#91 execution remains the causal RED authority for restoring this semantic. Current-owner production commit `3f84f02b46e44aac61092a520fcbc35cb05aa2f1` performs the minimum repair on the evolved #127 source: its complete source diff is one added `"--".to_owned()` immediately before `request.image_reference.clone()`. It changes no network selector, cleanup/lifecycle authority, typed spawn mapping, digest validation, image text, command entries, or no-shell behavior.

Commit `6a763370250957c6fc92108bca1e27a9146bc48c` removes the temporary one-shot source-fix workflow after the direct exact-blob write path became available. The workflow is therefore not part of current release authority. Older source-fix runs from superseded heads cannot establish current branch status or safely move the current ref.

The first CI materialized after source repair plus workflow removal is `35203360276`. Its verify, coverage, branch-coverage, hosted negative rootless/AppArmor, and positive-LSM jobs were still pre-runner queued when this record was refreshed. Do not call #133 current-head GREEN until an unchanged descendant containing this doctoring update executes the focused witness and the full required gates.

## Applied minimum repair

The current #133 source contains one literal `--` immediately before `request.image_reference` in the application-service `podman create` argv. Preserve the current image value, command entries, network selector, isolation flags, digest validation, `--pull=never`, lifecycle/cleanup authority and no-shell contract.

Do not shell-join values, escape or rewrite the image identity, restore generated-name destructive authority, or transplant the stale #91/#14 tree wholesale. Tightening repository-name grammar may be useful defense in depth, but it is not a substitute for an unambiguous infrastructure CLI boundary.

## Rejected alternatives

- Joining image/command text into one shell string enlarges the injection boundary and breaks direct argv.
- Escaping or prefixing the image token mutates workload identity instead of delimiting provider syntax.
- Relying on current repository validation is insufficient because the demonstrated option-shaped token is accepted, and future validation changes must not reopen the CLI boundary.
- Treating a backend parse error as containment does not prove the validated image remains the actual image operand.
- Folding this into #46 or #47 would conflate applied image-digest attestation or ENTRYPOINT semantics with provider CLI parsing.

## Verification and release gate

Keep #90, #132 and #133 open until one unchanged current canonical-owner exact head contains the source repair and this current doctoring record, executes the focused #133 witness GREEN, and proves all valid #127 network/lifecycle deltas remain intact. That exact head must also reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy with warnings denied, rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security gates, applicable real rootless/positive-LSM evidence, protected integration and immutable publication evidence.

## References

MITRE. (2026). *CWE-88: Improper neutralization of argument delimiters in a command ('Argument Injection')* (CWE 4.20). https://cwe.mitre.org/data/definitions/88.html

Podman Authors. (2026). *Podman container create command source*. https://github.com/podman-container-tools/podman/blob/main/cmd/podman/containers/create.go

pflag Authors. (2026). *pflag: POSIX/GNU-style command-line flags*. https://github.com/spf13/pflag

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
