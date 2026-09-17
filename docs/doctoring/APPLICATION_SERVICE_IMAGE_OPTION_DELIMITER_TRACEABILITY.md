# Application-service image option-delimiter traceability

## Problem and bounded-context ownership

Canonical application-service successor #127 exact `1a3cd0427bbf99e41c1d8bd4089747561237071f` validates a consumer-supplied immutable image reference, then places that string directly after Podman `create` options and before consumer command argv. The plan has no explicit `--` terminator before the image positional operand.

The application-service request validator currently accepts a repository component beginning with `-` and containing `=` when the overall value ends in a valid lower-case `@sha256:<64-hex>` digest. Therefore a Podman-shaped token such as `--annotation=qsr.boundary=value@sha256:<digest>` is accepted as an image reference even though it begins with CLI option syntax.

This is an infrastructure-adapter argv-boundary defect under the `application_service` Supporting context. It does not transfer Podman option syntax into the domain model and does not change consumer authorization. The invariant is that the digest-validated workload identity must remain Podman's image positional operand.

## Parser evidence

Podman's current `cmd/podman/containers/create.go` declares `create [options] IMAGE [COMMAND [ARG...]]`. Its `createFlags` function calls `flags.SetInterspersed(false)`. pflag documents `--` as the flag-parsing terminator. With no explicit terminator, an accepted leading-dash image token is still eligible for option parsing before Podman reaches its first positional image argument.

That matters beyond a cosmetic parse failure. The request also contains consumer-controlled command argv. If the image token is consumed as a valid Podman option, the next command token can become Podman's first positional `IMAGE`, so the SHA-256 identity validated by the runtime no longer necessarily describes the workload operand presented to Podman.

## Checked-in RED

Issue #132 owns this defect. Test-only child `088fd10e794b5c02a13ed45e082d509dc77ccfb9` adds `tests/podman_application_service_option_delimiter_red.rs` on exact #127 parent `1a3cd0427bbf99e41c1d8bd4089747561237071f`.

The witness:

- builds an otherwise-valid request with `--annotation=...@sha256:<digest>` as the accepted image reference;
- proves request validation succeeds before inspecting backend argv;
- supplies a following image-shaped consumer command token to make positional displacement explicit;
- requires exactly one literal `--` immediately before the validated image reference;
- requires command argv after the image to remain entry-preserving.

Current production contains no such terminator, so this is a checked-in RED candidate. It is not causal executed RED until an exact CI job reaches the assertion and fails for the missing delimiter.

## Minimum repair after executed RED

The smallest causal repair is one literal `--` immediately before `request.image_reference` in the application-service `podman create` argv. Preserve the existing image value, command entries, network selector, isolation flags, digest validation, `--pull=never`, and no-shell contract.

Tightening repository-name validation to reject leading-dash or otherwise non-OCI grammar may be useful defense in depth, but validator hardening alone is not chosen as the causal repair. The backend argv boundary should remain unambiguous even if accepted domain syntax evolves.

## Rejected alternatives

- Joining image/command text into one shell string: breaks direct argv and creates a larger injection boundary.
- Escaping or prefixing the image token: mutates workload identity rather than delimiting CLI syntax.
- Relying on current repository validation: the demonstrated accepted token contradicts that assumption and future validation changes should not reopen CLI ambiguity.
- Treating a backend parse error as sufficient containment: fail-closed errors do not prove that the validated image remains the actual image operand for every accepted request.
- Moving this defect into #46 or #47: applied image-digest verification and ENTRYPOINT semantics are separate controls after/create-process interpretation.

## Verification and release gate

After causal RED execution, the one-token production repair must make the focused witness GREEN and retain all #127 network/lifecycle evidence. One unchanged exact head must then pass repository validation, rustfmt, full locked workspace/all-target tests, Clippy with warnings denied, rustdoc with warnings denied, complete owned-production statement/function/region/branch/edge coverage, qualifying review/security gates, applicable real rootless/positive-LSM evidence, protected integration, and immutable publication evidence before release authority.

## References

Podman Authors. (2026). *Podman container create command source*. https://github.com/podman-container-tools/podman/blob/main/cmd/podman/containers/create.go

pflag Authors. (2026). *pflag: POSIX/GNU-style command-line flags*. https://github.com/spf13/pflag

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
