# Application-service pre-attestation execution traceability

## Current finding

Issue #128 records a P0 execution-order defect in the application-service Podman lifecycle. The current owner path creates an application container, starts the exact acquired container ID, and only then calls `verify_effective_isolation`. At that point the image's untrusted process can already have executed. A later `IsolationVerificationFailed` plus cleanup proves rejection/teardown, not prevention of pre-attestation side effects.

Test-only owner witness `tests/podman_application_service_pre_attestation_execution_owner_red.rs` models that boundary without banning `podman start` itself. Its fake backend marks the consumer as directly armed only when container creation leaves the image payload as the initial OCI process without an explicit runtime-owned entrypoint hold. `start` then records a consumer side effect. The following container inspection deliberately contradicts the required capability state. Acceptance requires the sandbox to fail effective attestation while the consumer-side-effect marker remains absent.

This document and the witness do not make the current head GREEN. The causal RED must execute on its exact SHA before production repair.

## Authority separation

The application-service profile needs four states that must not be conflated:

- **created sandbox** — runtime objects exist, but hostile service code has no execution authority;
- **hold process running** — only a verified runtime-owned hold primitive may execute so live process isolation can be measured;
- **attested sandbox** — configured and live/effective isolation evidence has been accepted for the exact acquired container/network identities;
- **released service** — the one-time runtime-owned release transition has completed and the hostile service payload may run, after which readiness may be evaluated.

`cleanup after rejected attestation` is not equivalent to `payload was never released`.

## Selected direction and rejected shortcuts

The selected architectural direction is a trusted hold/attest/release boundary analogous to the command-runtime control of issue #25, but application-service truth remains owned by the application-service runtime. Reuse of the command runtime's gate is allowed only after the relevant gate capability is ordinarily integrated into a stable owner contract; copying source from a mutable sibling branch or depending on a sibling PR head is not acceptable.

The following are rejected:

- allowing the hostile image process to run briefly and relying on cleanup after failed attestation;
- checking only create argv or static image metadata as a substitute for live effective evidence;
- asking the hostile image/entrypoint to self-report that it is held;
- sleep/retry windows intended to make inspection happen "quickly enough" after start;
- moving the same verification after readiness or lease publication;
- weakening seccomp/capability/LSM/network/resource predicates so pre-release evidence becomes easier to obtain.

A causal production repair must preserve the existing exact container/network ownership work (#40, #42, #48), effective attachment and foreign-safe cleanup work (#22, #41), and the dedicated positive-LSM/runtime evidence gates.

## Reproducible owner-path evidence

Current source ordering is observable in `RootlessPodmanAdapter::launch_at`: exact container `start` precedes `verify_effective_isolation`. The owner witness deliberately returns `EffectiveCaps=["CAP_NET_RAW"]` after start, while other prerequisite evidence remains positive enough to reach `all_capabilities_dropped`. The test requires no `port`/readiness call after the contradiction and, critically, no consumer side-effect marker before that rejection.

The witness is an active normal-Cargo RED. Do not mark it `#[ignore]`; exact-head execution is required to establish the causal failure boundary.

## Acceptance after causal RED

A minimum safe composition is:

`verified hold primitive -> create held sandbox -> acquire exact network/container identities -> start hold primitive only -> configured/live effective isolation attestation -> one-time release -> trusted release acknowledgement -> readiness -> lease publication`.

Release failure, acknowledgement contradiction, or any pre-release attestation failure must clean only runtime-owned exact identities and must leave the hostile service payload unreleased. Real rootless acceptance must demonstrate this with a workload whose attempted pre-release side effect is externally observable and remains absent, together with positive effective LSM/network/resource evidence.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: State*. https://github.com/opencontainers/runtime-spec/blob/main/specs-go/state.go

Podman Authors. (2026). *podman-start — Start one or more containers*. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

ContextualWisdomLab. (2026). *Issue #25: P0 security — prevent command payload execution before effective isolation attestation*. `ContextualWisdomLab/quarantine-sandbox-runtime`.

ContextualWisdomLab. (2026). *Issue #128: P0 application-service — hold hostile payload until effective isolation attestation*. `ContextualWisdomLab/quarantine-sandbox-runtime`.
