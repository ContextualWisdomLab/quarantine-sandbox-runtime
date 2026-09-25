# Application-service image option-delimiter traceability

## Current regression and bounded-context ownership

Canonical application-service/network parent #127 is exact `4306462ddbd90ce137074e70af7c9c0bbee18d9e`. Draft successor #133 has now adopted that exact parent non-force while preserving its option-delimiter production/test/documentation delta. `application_service` owns immutable workload intent; `infrastructure::podman` owns translation into Podman's CLI grammar. The invariant is that the digest-validated workload identity remains Podman's image positional operand and cannot be reinterpreted as provider control syntax.

The request validator accepts a repository component beginning with `-` and containing `=` when the overall value ends in a valid lower-case `@sha256:<64-hex>` digest. A Podman-shaped token such as `--annotation=qsr.boundary=value@sha256:<digest>` therefore reaches the infrastructure plan as domain-valid image data.

This is not a new control. It is a succession regression of reopened issue #90 / historical PR #91.

## Historical causal evidence

PR #91 supplied the original focused application-service witness. Native CI `34202917157`, coverage job `101985631210`, executed `tests/podman_option_terminator_red.rs` and observed the policy-SHA label immediately before a domain-valid option-shaped image instead of literal `--`.

Minimum production commit `1b1026a92b7d9ef23a709564c1e22aa0421179ff` inserted exactly one literal `--` immediately before `request.image_reference` while preserving the image and following command argv entry-for-entry.

Canonical command-runtime #14 later succeeded the application-service delimiter, the focused regression and refreshed `PODMAN_OPTION_TERMINATOR_TRACEABILITY.md`, and added independent runtime-gate/command-path coverage. Exact #14 `6df79290321f697b2a23c9cf246575a4d13c5740`, CI `34489161644`, had verify `102911078197` GREEN through exact checkout, repository policy, rustfmt, full locked workspace/all-target tests, Clippy and rustdoc; hosted negative rootless/AppArmor `102911078027` was also GREEN.

Issue #90 and PR #91 were therefore legitimately completed at that point. Fresh inspection of #127 showed the verified delimiter had subsequently fallen out of the canonical application-service/network ancestry, so #90 was reopened.

## Parser evidence

Podman's current `cmd/podman/containers/create.go` declares `create [options] IMAGE [COMMAND [ARG...]]` and configures pflag with `SetInterspersed(false)`. pflag treats `--` as the explicit flag-parsing terminator. A leading-dash token presented where the image should begin is still eligible for option parsing until that boundary is terminated.

The request also contains consumer-controlled command argv. If the accepted image token is consumed as a Podman option, a following command token can become the first positional `IMAGE`. The SHA-256 identity validated by the runtime can then cease to describe the workload operand presented to Podman.

## Current-owner replay and repair

Issue #132 / Draft #133 is the current regression-instance lane. Test-only `088fd10e794b5c02a13ed45e082d509dc77ccfb9` added `tests/podman_application_service_option_delimiter_red.rs` on the earlier #127 exact `1a3cd0427bbf99e41c1d8bd4089747561237071f`.

The witness:

- builds an otherwise-valid request with `--annotation=...@sha256:<digest>` as the image reference;
- independently proves request validation succeeds before inspecting backend argv;
- supplies a following image-shaped consumer command token so positional displacement is explicit;
- requires exactly one literal `--` immediately before the exact validated image reference;
- requires command argv after the image to remain entry-preserving.

Historical #90/#91 execution remains the causal RED authority for restoring this semantic. Current-owner production commit `3f84f02b46e44aac61092a520fcbc35cb05aa2f1` performs the minimum repair on the evolved application-service source: its complete production diff is one added `"--".to_owned()` immediately before `request.image_reference.clone()`. It changes no network selector, cleanup/lifecycle authority, typed spawn mapping, digest validation, image text, command entries, or no-shell behavior.

Commit `6a763370250957c6fc92108bca1e27a9146bc48c` removes the temporary one-shot source-fix workflow. The workflow is not part of release authority.

Exact predecessor `4d5794b1a41d49ee80e3f116db705538e247606f`, CI `35203507971`, has executed. Exact checkout, dependency lock, repository policy and coverage-parser tests passed; verify then failed at `cargo fmt --check` before Rust tests, lint or rustdoc. Hosted negative rootless/AppArmor was GREEN. Coverage and branch-coverage reached Rust evidence generation but failed there; positive SELinux never acquired a runner and was cancelled after the queue window. This execution is not focused semantic GREEN because the option-delimiter test did not run in verify.

Formatter-only descendant `8265dd5e2a1a19c4de2e5b647c3a5ff0788fdf09` applies rustfmt's multiline layout to the long `iter().filter().count()` assertion in the focused regression. Production source and assertions are unchanged.

GitHub then materialized merge commit `8d18fb5701fb7b5690fed270a24c9d55b2785cd1` with parents current #127 `4306462ddbd90ce137074e70af7c9c0bbee18d9e` and formatter-clean #133 `8265dd5e2a1a19c4de2e5b647c3a5ff0788fdf09`. The #133 branch was advanced to that commit with a non-force fast-forward. The resulting tree adopts #127's acquired-network-ID fixture repair and preserves all three #133-owned files; no production option-delimiter delta or parent fixture delta was discarded.

No predecessor CI conclusion transfers after that ancestry movement. The current descendant must execute independently.

## Applied minimum repair

The current #133 source contains one literal `--` immediately before `request.image_reference` in the application-service `podman create` argv. Preserve the current image value, command entries, network selector, isolation flags, digest validation, `--pull=never`, lifecycle/cleanup authority and no-shell contract.

Do not shell-join values, escape or rewrite the image identity, restore generated-name destructive authority, or transplant the stale #91/#14 tree wholesale. Tightening repository-name grammar may be useful defense in depth, but it is not a substitute for an unambiguous infrastructure CLI boundary.

## Rejected alternatives

- Joining image/command text into one shell string enlarges the injection boundary and breaks direct argv.
- Escaping or prefixing the image token mutates workload identity instead of delimiting provider syntax.
- Relying on request validation is insufficient because the demonstrated option-shaped token is accepted, and future validation changes must not reopen the CLI boundary.
- Treating a backend parse error as containment does not prove the validated image remains the actual image operand.
- Folding this into #46 or #47 would conflate applied image-digest attestation or ENTRYPOINT semantics with provider CLI parsing.

## Verification and release gate

Keep #90, #132 and #133 open until one unchanged current canonical-owner exact head contains the source repair and this current doctoring record, executes the focused #133 witness GREEN, and proves all valid #127 network/lifecycle deltas remain intact. That exact head must also reacquire repository validation, rustfmt, full locked workspace/all-target tests, Clippy with warnings denied, rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security gates, applicable real rootless/positive-LSM evidence, protected integration and immutable publication evidence.

## References

MITRE. (2026). *CWE-88: Improper neutralization of argument delimiters in a command ('Argument Injection')* (CWE 4.20). https://cwe.mitre.org/data/definitions/88.html

Podman Authors. (2026). *Podman container create command source*. https://github.com/podman-container-tools/podman/blob/main/cmd/podman/containers/create.go

pflag Authors. (2026). *pflag: POSIX/GNU-style command-line flags*. https://github.com/spf13/pflag

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
