# Application-service lifecycle ownership traceability

## Decision

Application-service container lifecycle authority is bound to the exact long container ID admitted from the successful `podman create` invocation. The generated `qsr-app-*` name remains correlation/audit metadata and may continue to back the public `sandbox_id` contract, but it is not post-create lifecycle or destructive authority.

This document distinguishes the executed Issue #40 RED state from the repaired implementation. At the RED head, the adapter parsed the created container ID but then re-resolved the generated name for post-create lifecycle operations. Current production no longer does that: `launch_at` carries the admitted `container_id` into start/inspect/top/port, failure cleanup, readiness cleanup, and private cleanup authority; `terminate_at` uses that private exact-ID authority for stop/remove. Generated names are not a fallback destructive selector.

## Authority chain

| Evidence | What it establishes | Runtime consequence |
| --- | --- | --- |
| Successful `podman create` identity evidence | The invocation acquired one concrete container identity. | Admit an exact long ID before post-create lifecycle use. |
| Runtime-owned create receipt | Successful create may need exact-ID recovery even when stdout is malformed. | Reconcile trustworthy receipt/stdout evidence; never infer destructive authority from `qsr-app-*`. |
| Podman container inspect | Inspect returns the concrete container `Id`. | Address inspect by admitted ID and require returned-ID consistency. |
| NIST SP 800-190 | Runtime lifecycle/isolation controls must preserve container security boundaries. | Cleanup/control operations must not cross invocation ownership boundaries. |
| Issue #40 / Draft #21 causal RED | Immediate name rebinding after successful create could redirect post-create operations. | Generated-name lifecycle selection is a security defect after ID acquisition. |
| Issue #113 / Draft #21 receipt repair | Successful create with malformed stdout can otherwise leave ambiguous cleanup authority. | Recover only an exact runtime-owned receipt identity; fail closed without trustworthy authority. |

The application-service destructive-authority grammar is intentionally narrower than human-facing Podman selectors: current repository policy admits exactly 64 lower-case hexadecimal bytes. Upper-case, short-ID, name-like, non-hex, padded, missing, mismatched, unreadable, or invalid-UTF-8 proof does not become lifecycle authority. Untrusted identity is not normalized into an accepted spelling.

## Executed RED evidence

`tests/podman_application_service_post_create_ownership_red.rs` models an otherwise-positive application-service launch in which the fake backend returns one fixed long owned ID from `create` and treats lifecycle operations addressed by the generated name as a foreign-resource side effect. The same operations addressed by the exact ID remain valid.

The causal #40 lineage proved that the pre-repair implementation could select the generated name after exact ID acquisition. Exact predecessor `a7753b6d7219ae85d9fd97d93c809e9e3019ecd5`, native CI `34308490681`, reached the ownership regression in coverage and branch-coverage lanes and failed with cleanup/lifecycle selection evidence rather than merely inferring the defect from source.

The RED requires the public `sandbox_id` to remain `qsr-app-*` correlation metadata while `start`, `container inspect`, `top`, `port`, `stop`, and `rm` use the acquired long ID. This separates consumer-facing correlation identity from private backend lifecycle authority.

## Current repair

The minimum #40 production repair is retained in current #21 ancestry:

- `a5d0ef1d5bcd2626ea12b7f6552d2ecf254a1616` separated runtime cleanup selection from public correlation identity;
- `7cd4ffa6dff56e826aa432a6d348e154c3b4873f` bound successful post-create start, inspect/top, port, launch/readiness failure cleanup, and later termination to the admitted container ID;
- later #113 work added a private runtime-owned create receipt so malformed-successful-create cleanup can recover exact authority without falling back to a generated name;
- causal RED `db42f814265dbf99c807b843000dc8aa93cc8ba7` exposed inconsistent stdout/receipt grammar for upper-case hexadecimal, and minimum repair `31cc048a8a94e73dca49ee5d2c097582859b039a` aligned both channels to the same exact 64-character lower-case hexadecimal contract.

Current exact PR candidate `f5e23f6611f7ee8782fba25f32de7fc5158b4f56`, native CI `34669059735`, made hosted verify, full tests, Clippy/rustdoc, production coverage admission, branch coverage admission, and hosted negative rootless/AppArmor GREEN before this documentation correction. That candidate reported lines `2123/2123`, functions `205/205`, branches `462/462`; raw LLVM regions remained diagnostic `2816/2819` while the canonical source-region admission passed. These predecessor results are causal evidence and do not transfer to a documentation descendant without fresh exact-head execution.

## Remaining gates

This repair does not by itself establish live confinement or release authority. Dedicated positive effective-LSM remains a separate self-hosted gate. Issue #113 remains open until the exact candidate has qualifying independent review/security, dependency-safe protected integration, positive runtime evidence, and immutable release/SBOM/provenance/reproducibility/rollback evidence.

Do not replace exact-ID authority with larger random names. Do not weaken effective isolation, network binding, resource attestation, LSM proof, cleanup precedence, or caller-scoped idempotency to simplify lifecycle ownership.

## References

Podman Authors. (2026). *podman-container-inspect — Display a container’s configuration*. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman Authors. (2026). *podman-run — Run a command in a new container*. https://docs.podman.io/en/stable/markdown/podman-run.1.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
