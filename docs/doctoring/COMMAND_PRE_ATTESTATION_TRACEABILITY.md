# Command pre-attestation execution traceability

## Status

Issue #25 remains an open P0 on canonical command-runtime PR #14 and ADR-0008 remains Proposed. The original pre-attestation payload-execution defect is an executed causal RED. A later `podman init` capability RED has also executed and has a minimum partial production repair, but that repair establishes only an initialized/configuration hold point; it is not release-grade effective-process attestation.

Original causal evidence on 2026-09-09 KST:

- source head: `ed9318ba96d876341866d84a892d0145e12f6469`;
- native CI: `34340070150`;
- verify: `102428599037`, failed at `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` after the preceding command-runtime and application-service suites passed;
- branch coverage: `102428599228`, failed at the same hostile RED;
- hosted negative rootless/AppArmor: `102428599461`, passed;
- dedicated positive-LSM: `102428599423`, queued at the time of the causal RED and therefore not acceptance evidence.

The failing fixture records the consumer command sentinel when `podman create` receives it, records a payload side effect when `podman start` is called, and then withholds live process attestation. `run_command_at` returns an error and attempts cleanup, but the payload marker already exists. The failure therefore demonstrates a real ordering defect: fail-closed validation after `start` cannot undo hostile code that already became runnable.

The next capability slice narrowed an earlier safe rejection point:

- test-bearing init-hold RED: `9e36766abc0a553f45c62fa327b34f1315dc0db9`;
- executable predecessor after immutable-fixture prerequisites: `eb2330ffb6689b07706a305e55c81871414e9fe5`;
- native CI: `34353894509`;
- intended failure: both `podman_command_execution_init_hold_capability_red` cases observed `info -> create -> start -> container inspect -> rm`, never invoked `podman init`, and recorded the payload side effect. One case required an unavailable/failed init primitive to stop release; the other required invalid read-only-rootfs configuration to be rejected while initialized but not started;
- minimum production repair: `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1`;
- formatter-only follow-up: `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e`.

The repaired owner path now performs `create -> acquire exact ID -> podman init <ID> -> static/configuration verification while held -> podman start <ID> -> live effective-process verification`. Later fixture work made the fake backend explicitly init-capable and repaired the deterministic start-failure fixture without weakening pre-start verification. The purpose-specific source materializer used to apply that fixture repair has been retired after the source delta was materialized; it is not part of the runtime architecture.

## DDD owner and invariant

`sandbox_execution` is the Core bounded context and owns the execution/isolation lifecycle invariant. Podman is an infrastructure adapter. The public command contract remains owned by `application_service`, but a consumer must not have to understand or reproduce Podman/OCI lifecycle details.

Invariant:

> A hostile consumer process MUST NOT become runnable until the runtime has positively established the effective isolation evidence required by the selected execution profile. Failure to establish that evidence MUST terminate and clean up the exact runtime-owned sandbox without releasing the consumer process.

Configured intent and static backend inspection are not substitutes for observed effective process confinement.

## Current causal mechanism

Current `RootlessPodmanAdapter::run_command_at` no longer starts immediately after create. It encodes the complete validated consumer argv as the OCI entrypoint, creates the container, admits the exact acquired container ID, invokes `podman init` on that ID, and verifies static/configuration controls while the container remains initialized but not started. Only after that early gate passes does it invoke `podman start`, followed by the existing live `container inspect` plus `podman top` seccomp/LSM/capability verifier.

This removes the already-executed counterexample in which a backend lacking the init hold was silently used, and it rejects configuration that already disproves required controls before `start`. It does not establish the invariant above. Effective seccomp/LSM/capability state for the consumer process may be finalized at or near exec, after the init/static gate and before the post-start live sample. The original payload-side-effect P0 therefore remains open.

The existing live-attestation fail-closed behavior must remain. The remaining repair is an additional effective pre-payload execution boundary, not a fallback to static evidence.

## Standards analysis

The OCI Runtime Specification gives a useful but narrow ordering guarantee. During `create`, the runtime environment is established while the user-specified program must not run. Podman documents `podman init` as initializing one or more containers and performing work required to start them without starting them. During OCI `start`, `startContainer` hooks execute before the user-specified process. OCI also states that `createRuntime` and `createContainer` are underspecified with respect to cgroups and SELinux/AppArmor being applied, so those earlier hook stages cannot by themselves satisfy this repository's effective-isolation claim.

`podman init` is therefore a valid hold/configuration primitive and an earlier fail-closed point, but not effective-process attestation. `startContainer` remains a plausible primitive for an effective hold/attestation boundary, but it is not itself the complete product solution. Its hook path resolves and executes in the container namespace. An arbitrary hostile tool image cannot be assumed to contain a trusted attestor. A production hook-based design would need a runtime-owned, immutable, architecture-compatible attestor delivered through a separately bounded and attested path, plus a bounded evidence/release channel and explicit backend capability detection.

Podman's `--hooks-dir` can inject OCI hooks, but supporting hook configuration is not proof that a particular runtime/backend combination supplies the exact effective-state evidence this product requires. Real rootless acceptance on an eligible LSM backend remains mandatory.

NIST SP 800-190 treats the container runtime and its isolation mechanisms as security-critical parts of the container platform. That supports fail-closed runtime ownership but does not prescribe this implementation-specific hold/attestor design.

## Alternatives considered

### Reorder static `container inspect` before `podman start`

Rejected as a complete P0 solution. It can bind configured/applied container state but cannot establish live process seccomp, LSM and capability evidence. The current init-held static verification deliberately uses this information only as an early contradiction/rejection gate.

### Treat `podman init` plus static inspection as full GREEN

Rejected. The executed init-hold RED proves `init` is useful and must fail closed when unavailable, but initialized/static state does not prove the effective controls on the consumer process that will execute later.

### Start an inert process and later run the consumer through plain `podman exec`

Rejected as insufficient. A plain exec transfers the same question to the exec process: the consumer can become runnable before equivalent effective confinement for that process has been positively established.

### Use `createRuntime` or `createContainer` hook alone

Rejected for the P0 proof. OCI explicitly allows cgroups and SELinux/AppArmor setup to be incomplete at these stages.

### Use `startContainer` as the complete repair without additional trust controls

Rejected. The stage ordering is useful, but the hook executable is resolved inside the container namespace. Trusting image-supplied code would invert the security boundary.

### Runtime-owned pre-payload attestor plus fail-closed capability contract

Selected for further RED-driven validation, not yet Accepted as an implementation. The runtime must prove that a supported Podman/OCI backend can keep the consumer process held beyond the current init/static gate, execute a runtime-owned attestor in the required effective context, return bounded evidence, and release the consumer only after positive policy evaluation.

## Next RED before full production GREEN

Add a checked-in effective hold/attest/release capability regression that starts from the current init-held design and distinguishes runtime-owned attestation code from the hostile consumer payload. It must require fail-closed behavior when the effective attestation primitive is unavailable, malformed, cannot run in the required context, or cannot produce every required bounded evidence item. It must also prove that the consumer side-effect marker remains absent until the runtime explicitly releases execution after positive policy evaluation.

Only after that RED executes for its intended cause may production add the smallest runtime-owned effective hold/attest/release adapter. The original payload-side-effect RED must then become GREEN on the same implementation. Real rootless Podman E2E on an eligible LSM backend must independently verify the effective process boundary before merge or release.

Applied-state gaps #32, #35 and remaining #39 must compose into the pre-release attestation path; they are not waived by the lifecycle repair. Exact acquired-ID cleanup authority, source-mount integrity, egress denial, resource bounds, timeout semantics and bounded output remain unchanged.

## Release effect

No version, tag, package, GitHub Release or immutable consumer publication is authorized while #25 remains open without same-head effective hold/attest/release GREEN and real positive-LSM acceptance. Hosted negative confinement is not positive LSM acceptance, queued positive-LSM evidence is not transferable evidence, and the init-held static/configuration gate is not relabeled effective isolation.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Open Container Initiative. (2026). *Open Container Initiative runtime specification: POSIX-platform hooks*. https://github.com/opencontainers/runtime-spec/blob/main/config.md

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Podman Authors. (2026). *podman — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190