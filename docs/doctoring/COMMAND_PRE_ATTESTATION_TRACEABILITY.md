# Command pre-attestation execution traceability

## Status

Issue #25 is an executed P0 RED on canonical command-runtime PR #14. It is not release-ready and ADR-0008 remains Proposed.

Exact causal evidence on 2026-09-09 KST:

- source head: `ed9318ba96d876341866d84a892d0145e12f6469`;
- native CI: `34340070150`;
- verify: `102428599037`, failed at `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` after the preceding command-runtime and application-service suites passed;
- branch coverage: `102428599228`, failed at the same hostile RED;
- hosted negative rootless/AppArmor: `102428599461`, passed;
- dedicated positive-LSM: `102428599423`, queued at the time this trace was written and therefore not acceptance evidence.

The failing fixture records the consumer command sentinel when `podman create` receives it, records a payload side effect when `podman start` is called, and then withholds live process attestation. `run_command_at` returns an error and attempts cleanup, but the payload marker already exists. The failure therefore demonstrates a real ordering defect: fail-closed validation after `start` cannot undo hostile code that already became runnable.

## DDD owner and invariant

`sandbox_execution` is the Core bounded context and owns the execution/isolation lifecycle invariant. Podman is an infrastructure adapter. The public command contract remains owned by `application_service`, but a consumer must not have to understand or reproduce Podman/OCI lifecycle details.

Invariant:

> A hostile consumer process MUST NOT become runnable until the runtime has positively established the effective isolation evidence required by the selected execution profile. Failure to establish that evidence MUST terminate and clean up the exact runtime-owned sandbox without releasing the consumer process.

Configured intent and static backend inspection are not substitutes for observed effective process confinement.

## Current causal mechanism

Current `RootlessPodmanAdapter::run_command_at` encodes the entire validated consumer argv as the OCI entrypoint during `podman create`, starts that container, and only then invokes `verify_command_isolation`. The live check samples Podman inspect plus process evidence after start. This sequence is sufficient to reject an inadequately confined result, but it is too late to prevent a hostile payload from executing before the rejection.

The existing live-attestation fail-closed behavior must remain. The repair is an additional pre-payload execution boundary, not a fallback to static evidence.

## Standards analysis

The OCI Runtime Specification gives a useful but narrow ordering guarantee. During `create`, the runtime environment is established while the user-specified program must not run. During `start`, `startContainer` hooks execute before the user-specified process. OCI also states that `createRuntime` and `createContainer` are underspecified with respect to cgroups and SELinux/AppArmor being applied, so those earlier hook stages cannot by themselves satisfy this repository's effective-isolation claim.

`startContainer` is therefore a plausible primitive for a hold/attestation boundary, but it is not itself the complete product solution. Its hook path resolves and executes in the container namespace. An arbitrary hostile tool image cannot be assumed to contain a trusted attestor. A production hook-based design would need a runtime-owned, immutable, architecture-compatible attestor delivered through a separately bounded and attested path, plus a bounded evidence/release channel and explicit backend capability detection.

Podman's `--hooks-dir` can inject OCI hooks, but supporting hook configuration is not proof that a particular runtime/backend combination supplies the exact effective-state evidence this product requires. Real rootless acceptance on an eligible LSM backend remains mandatory.

NIST SP 800-190 treats the container runtime and its isolation mechanisms as security-critical parts of the container platform. That supports fail-closed runtime ownership but does not prescribe this implementation-specific hook design.

## Alternatives considered

### Reorder static `container inspect` before `podman start`

Rejected. It can bind configured/applied container state but cannot establish live process seccomp, LSM and capability evidence. It would weaken the existing security boundary.

### Start an inert process and later run the consumer through plain `podman exec`

Rejected as insufficient. A plain exec transfers the same question to the exec process: the consumer can become runnable before equivalent effective confinement for that process has been positively established.

### Use `createRuntime` or `createContainer` hook alone

Rejected for the P0 proof. OCI explicitly allows cgroups and SELinux/AppArmor setup to be incomplete at these stages.

### Use `startContainer` as the complete repair without additional trust controls

Rejected. The stage ordering is useful, but the hook executable is resolved inside the container namespace. Trusting image-supplied code would invert the security boundary.

### Runtime-owned pre-payload attestor plus fail-closed capability contract

Selected for further RED-driven validation, not yet Accepted as an implementation. The runtime must first prove that a supported Podman/OCI backend can hold the consumer process, execute a runtime-owned attestor in the required effective context, return bounded evidence, and release the consumer only after positive policy evaluation.

## Next RED before production GREEN

Add a checked-in backend-capability regression that models a supported hold/attest/release primitive separately from the consumer payload and requires fail-closed behavior when the primitive is unavailable, malformed, or cannot produce the required evidence. The fixture must distinguish runtime-owned attestation code from consumer code and must not count fake/static state as real isolation acceptance.

Only after that capability RED executes for the intended cause may production add the smallest runtime-owned hold/attest/release adapter. The existing payload-side-effect RED must then become GREEN on the same implementation. Real rootless Podman E2E on an eligible LSM backend must independently verify the effective process boundary before merge or release.

Applied-state gaps #32, #35 and remaining #39 must compose into the pre-release attestation path; they are not waived by the lifecycle repair. Exact acquired-ID cleanup authority, source-mount integrity, egress denial, resource bounds, timeout semantics and bounded output remain unchanged.

## Release effect

No version, tag, package, GitHub Release or immutable consumer publication is authorized while #25 remains RED. Hosted negative confinement is not positive LSM acceptance, and queued positive-LSM evidence is not transferable evidence.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Open Container Initiative. (2026). *Open Container Initiative runtime specification: POSIX-platform hooks*. https://github.com/opencontainers/runtime-spec/blob/main/config.md

Podman Authors. (2026). *podman — Podman documentation*. https://docs.podman.io/en/latest/markdown/podman.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
