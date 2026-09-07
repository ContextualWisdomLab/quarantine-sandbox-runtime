# Podman option-boundary TRACEABILITY

Date: 2026-09-08
Status: RED-only design record for issue #90
Parent authority: root PR #1 exact `78281e244c530dcafb3368b9f1d9896e846206a9`

## Problem

`RootlessPodmanAdapter::plan_at` translates validated application-service intent into Podman's external CLI grammar. The current create argv places `request.image_reference` directly after runtime-owned options and then appends `request.command`. The image-reference validator requires an immutable lower-case SHA-256 digest but does not prohibit a registry/name component that begins with `-`.

Direct process argv removes shell interpretation; it does not terminate the downstream CLI parser's own option interpretation. A consumer value can therefore be valid domain data and still be option-shaped to Podman. That is an anti-corruption-layer defect: `application_service` should not need Podman-specific delimiter knowledge, and infrastructure must preserve consumer values as positional data rather than allow them to be reinterpreted as backend control options.

## RED

`tests/podman_option_terminator_red.rs` constructs an otherwise-valid digest-pinned image whose first repository component begins with `-`, plus command entries beginning with `-` and containing spaces. It requires:

- the exact image string to remain present in `PodmanLaunchPlan::container_create_args()`;
- `--` immediately before that image operand;
- every command entry after the image to remain an exact argv entry, without shell joining or sanitizing.

Current production should fail only the delimiter assertion because it appends the image directly after the final runtime-owned label value.

## Minimum causal GREEN

After exact RED execution, insert one literal `--` immediately before `request.image_reference` in the Podman create argv. Preserve all existing P0/runtime-owned options, the image string, and all subsequent command argv entries. Do not solve this by weakening the image grammar, shell-quoting values, joining argv into a string, or folding issue #47's separate OCI ENTRYPOINT/direct-command semantics into this repair.

## DDD / security boundary

`application_service` owns immutable-image and command intent. `infrastructure::podman` owns the Podman CLI ACL. The ACL must delimit its own control-plane options from consumer positional data. Issue #47 remains the owner for effective OCI ENTRYPOINT/command identity; issue #90 owns only the external Podman option/data boundary.

## Primary-source basis

Podman documents `podman create [options] image [command [arg...]]`, making the image the first positional operand after Podman-owned options. MITRE CWE-88 defines argument injection as failure to delimit intended options/arguments across a command boundary and explicitly identifies insertion of `--` before attacker-controlled positional input as a mitigation pattern.

### References

MITRE. (2026). *CWE-88: Improper neutralization of argument delimiters in a command ('Argument Injection')* (CWE 4.20). https://cwe.mitre.org/data/definitions/88.html

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

## Completion gate

Do not merge or mark this control GREEN until one unchanged exact head has first executed the focused RED for the intended missing-terminator cause and then executed the minimum repair with fmt/test/clippy/rustdoc/repository validation, exact 100% owned production statement/function/region/branch coverage, review/security/thread gates, dependency-safe parent integration, real Podman/effective-isolation evidence where applicable, protected-head integration, SBOM/provenance/reproducibility/rollback, and immutable publication.
