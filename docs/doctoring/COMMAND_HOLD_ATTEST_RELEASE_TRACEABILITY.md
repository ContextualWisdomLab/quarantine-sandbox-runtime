# Command hold/attest/release traceability

Status: **Proposed / P0 open**

This note records the current evidence and the next acceptance boundary for issue #25. It does not mark ADR-0008 Accepted and it does not authorize a release.

## Exact evidence

Canonical command-runtime PR #14 reached exact source `92fd2c9a2347c2ad191478526d99d93acf0f82ff` after adopting 17 intervening test/fixture commits without force-push or destructive rebase. Native CI run `34371816198` checked out that exact SHA with persisted checkout credentials disabled.

The branch-coverage job `102535096412` passed the command-runtime prerequisites before reaching the controlling P0 regression. In particular, the primary 15 command-execution tests, create-failure ownership, exact ENTRYPOINT argv, invocation identity, image-digest binding, init-hold capability, live-attestation fail-closed behavior, bounded log storage, malformed-create cleanup, exact mount-set, namespace configuration, strict output encoding, Podman 4.9 compatibility, and acquired-ID lifecycle ownership all passed. It then failed exactly at:

`podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation`

The fake backend records a consumer side effect when `podman start` releases the OCI process and then withholds live process evidence. Production fails and cleans up, but the hostile payload has already become runnable. This is a causal security RED, not an infrastructure or fixture failure.

The current production order remains:

`create(consumer argv) -> acquire exact ID -> podman init -> configured-state verification -> podman start -> live process verification`

`podman init` is a valid earlier hold point, but it does not make the future consumer process's effective seccomp/LSM/capability state observable. Therefore the existing partial repair is necessary but not sufficient.

## Primary authority

OCI defines `created` as a state in which the container process has not executed the user-specified program. During `create`, all configuration except `process` must be applied, `process.args` must not be applied until `start`, and remaining process properties may be applied during `create`. `start` runs the user-specified program. This means portable code cannot assume every process-specific security property is already final merely because the container has been initialized or created.

Podman documents `podman init` as preparing a container for start without starting it. It is suitable for inspecting configured state before release, not for relabeling configured state as effective process evidence.

Linux preserves seccomp filters across `execve(2)` when `execve` is permitted. `no_new_privs` cannot be unset and is preserved across `execve(2)`. The capability bounding set constrains capabilities that can be gained during `execve(2)`. These kernel properties make an exec-preserving trusted gate a viable architecture to test, provided the gate itself is runtime-owned and its delivery/release authority is proven.

## Decision under test

The preferred next architecture is a **runtime-owned execution gate** as the OCI user process:

1. Bind the exact acquired container ID to a runtime-owned, architecture-compatible gate executable instead of the hostile consumer argv.
2. Start the gate under the final container process controls while it remains blocked before executing consumer code.
3. Observe effective seccomp, capability and LSM evidence for that running gate through the backend's live process evidence path.
4. Evaluate the selected isolation policy against bounded evidence.
5. Release only after positive evaluation; the gate then replaces itself with the exact consumer argv via `execve`, preserving the relevant process restrictions.
6. Treat gate delivery, identity, evidence, release-channel or exec failure as fail-closed and perform exact-ID cleanup.

This is an architecture hypothesis until a real Podman/OCI capability RED proves it. The gate binary must not come from the hostile image, must not depend on a hostile-image interpreter, and must have an immutable runtime-owned identity that is checked before release. Architecture compatibility is part of the contract, not a deployment assumption.

## Rejected shortcuts

- Moving static `container inspect` earlier is insufficient because configured state is not the same as effective process state.
- `podman init` plus static evidence is only a pre-release contradiction check; it is not full effective attestation.
- Starting the hostile consumer and failing after `podman top` is too late; cleanup cannot undo execution.
- Plain `podman exec` after an inert hostile-image entrypoint does not establish a trusted gate and creates another executable authority surface.
- A shell/script or helper supplied by the hostile image cannot be the attestation authority.
- A `startContainer` hook alone is not accepted as proof of the later consumer process's exact effective state; it may be useful only as part of a separately proven runtime-owned mechanism.
- Retry, sleep, mutex serialization or weakened assertions are not fixes for the release-order defect.

## Next causal RED

Before production changes, add an integration/capability regression that runs against a supported rootless Podman/OCI environment and proves all of the following on one exact head:

- consumer argv is not the OCI program released at initial `start`;
- a runtime-owned gate becomes running while consumer side effects remain impossible;
- the gate's immutable identity and architecture compatibility are verified;
- effective seccomp, capability and LSM evidence can be observed while the gate is held;
- malformed/absent evidence, unavailable gate primitive, wrong gate identity, release-channel failure, or policy mismatch fails closed before consumer release;
- positive policy evaluation is the only transition that releases the gate;
- release preserves the exact consumer argv and the existing no-network, exact-mount, resource, timeout, output and exact-ID lifecycle contracts;
- the original payload-side-effect RED and init-hold regressions become GREEN on the same implementation;
- an eligible positive-LSM runner supplies independent real-runtime acceptance evidence.

A fake-only test may define deterministic contract edges, but it cannot by itself prove the backend primitive exists or that effective kernel state survives the gate-to-consumer transition.

## Release effect

No version, tag, package, GitHub Release, immutable consumer publication, protected merge, or descendant release claim is authorized while issue #25 lacks same-head hold/attest/release GREEN, dedicated positive-LSM evidence, ordinary review/security gates, and protected-head integration evidence.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Podman Authors. (2026). *podman-init — Initialize one or more containers*. https://docs.podman.io/en/latest/markdown/podman-init.1.html

Kerrisk, M. (Ed.). (2026). *seccomp(2) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/seccomp.2.html

Kerrisk, M. (Ed.). (2026). *PR_SET_NO_NEW_PRIVS(2const) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man2/PR_SET_NO_NEW_PRIVS.2const.html

Kerrisk, M. (Ed.). (2026). *capabilities(7) — Linux manual page*. Linux man-pages project. https://man7.org/linux/man-pages/man7/capabilities.7.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
