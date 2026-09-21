# Podman Network Attachment Traceability

Last reviewed: 2026-09-22 KST

## Scope

This note records the evidence boundary for the canonical application-service network-lifecycle successor. It distinguishes the invocation-local `qsr-net-*` correlation, the immutable Podman network ID admitted by the runtime, effective container attachment evidence, and destructive cleanup authority. It does not promote a candidate repair to GREEN before current exact-head execution.

## Current problem and executed RED

PR #127 exact `82afbd1366aa7076c97470f3626686b614c8d29b` executed in native CI run `35636592189` on 2026-09-22 KST. The earlier missing-create-receipt fixture prerequisite was repaired successfully: that witness passed, and hosted rootless/AppArmor negative execution was GREEN.

The verify lane then reached `tests/podman_application_service_network_acquired_id_owner_red.rs::acquired_network_identity_precedes_create_and_preserves_attachment_red` and failed for the intended semantic cause. The hostile container reported `NetworkSettings.Networks={"podman":{"NetworkID":"foreign"}}`; the runtime nevertheless returned `Ok(ApplicationServiceLease)` where the witness requires `IsolationVerificationFailed { control_name: "sandbox_network_binding" }`.

This is a production defect rather than runner or fixture noise. Current source already acquires a canonical backend network ID and rewrites container creation to `--network <acquired-id>`, but `verify_effective_isolation()` did not consume that admitted ID. It inspected the acquired container without deserializing `NetworkSettings.Networks`, then re-inspected the public generated network name for only `internal`/DNS state. Configuration intent therefore substituted for effective attachment proof.

The same hostile owner test also proves a second failure-path boundary. Once the network ID has been admitted, cleanup after an effective-attachment mismatch must stop/remove the exact acquired container and perform `podman network rm <acquired-id>` without network-level `--force`. A generated-name force removal is not equivalent authority.

Coverage and branch-coverage on `82afbd...` stopped on the same owner path. Dedicated positive SELinux remained queued without an eligible self-hosted runner. No predecessor GREEN transfers to a later exact.

## Minimum causal repair

Production commit `6629fd30fa7c011f5aaff1b19ec1b116db2696bd` adds effective attachment evidence to `ContainerInspection`, carries the admitted network ID into `verify_effective_isolation()`, and requires exactly one effective attachment whose `NetworkID` equals that admitted ID before process security checks, network-state checks, or readiness publication can continue. Missing `NetworkSettings`, an empty attachment set, a different ID, or an additional network all fail closed under `sandbox_network_binding`.

The verifier now uses the admitted network ID for the later network-state inspection rather than re-resolving `qsr-net-*`. This preserves immutable authority through the verification path and removes the post-admission public-name lookup from this owner path. Dependent PR #142 still must be restacked and executed before that historical RED can be claimed succeeded.

The same repair introduces `cleanup_admitted_network(<id>)`, which performs non-force exact-ID removal. Every partial-launch path after successful network-ID admission now carries that ID through cleanup. Container removal remains exact-ID `rm --force`; network removal is exact-ID and non-force. If initial network-ID admission itself fails, the runtime no longer promotes the generated correlation name to destructive cleanup authority; no network removal is attempted on that unadmitted path, leaving durable recovery to issue #141.

Fixture commits `43df86fc8d2e6493d14c90bf04080a0349eb9b91` and `463e6d9cc465f638f04c3e3cbe0e1b286b246838` update positive application-service and identity-race doubles with explicit effective `NetworkSettings.Networks[*].NetworkID` evidence and update post-admission partial-cleanup expectations to exact-ID/non-force semantics. Commit `580ff3d70c1da22707a1f172527878fa395a11a3` restores the unchanged fixed-width entropy boundary after the source edit and is the current code exact before this doctoring update.

The source repair intentionally does **not** claim the full network-lifecycle contract. Successful leases still expose the generated `qsr-net-*` correlation and their current private cleanup authority still derives from that public field; explicit termination therefore remains a separate RED requiring exact admitted ID plus non-force network removal. Creation-bound identity is also still unresolved: current acquisition is `network create` followed by a separately resolved name-based first inspection, so the create→first-inspect replacement witness remains the next earlier chronology gate.

## Authority model

The network lifecycle preserves separate concepts:

- `qsr-net-*`: invocation-local Podman network name and consumer-visible correlation evidence;
- admitted backend network ID: canonical lowercase 64-hex identity acquired before container creation;
- selected container `--network` argument: the admitted backend ID;
- `NetworkSettings.Networks` map key: backend-reported attached network name, not destructive authority;
- `NetworkSettings.Networks[*].NetworkID`: effective attachment identity that must equal the admitted ID;
- private cleanup authority: immutable admitted ID retained for post-admission failure cleanup and, after the later lease-authority repair, explicit termination.

No name, prefix, label, age, dangling state, `HostConfig.NetworkMode`, or consumer-visible lease correlation can substitute for the admitted backend ID.

## Remaining chronology

A dependency-safe integrated head must still prove, in order:

1. network identity is bound to the object created by this invocation rather than a same-name replacement between create and first inspection;
2. the admitted object proves expected generated name, canonical ID, `internal=true`, and DNS disabled before container creation;
3. failed identity admission authorizes no correlation-name deletion, with orphan reconciliation owned by issue #141;
4. the admitted ID remains continuous through effective verification;
5. exactly one effective container attachment carries that ID, with missing/different/additional attachment failing closed before readiness;
6. every post-admission partial failure removes the exact network ID without network-level force;
7. successful lease registration preserves public correlation separately from private exact cleanup authority;
8. explicit termination removes the exact admitted network ID without force and returns `CleanupFailed` rather than deleting foreign membership;
9. the create→first-inspect race is replaced by a creation-bound receipt or equivalent atomic backend authority; Podman 6.0.0 Libpod network-create response remains the verified candidate, subject to target rootless socket/permission/failure-semantics and Podman-machine/Colima portability evidence.

## Rejected alternatives

- Re-resolving `qsr-net-*` after an ID has already been admitted. The name is correlation metadata and can be rebound.
- Trusting only `HostConfig.NetworkMode`. Configuration intent is not effective attachment proof.
- Accepting absent `NetworkSettings`. Missing effective membership evidence must fail closed.
- Accepting one expected attachment plus additional networks. The application-service P0 contract requires exactly one effective attachment.
- Falling back from an admitted ID to generated-name cleanup. This discards stronger authority exactly when destructive action begins.
- Using `podman network rm --force` as a cleanup guarantee. Podman defines force removal to remove containers using the network; an in-use network must instead fail cleanup closed.
- Treating `podman network create` CLI stdout as the immutable ID. Podman documents that CLI success prints the newly created network name.
- Putting the acquired backend ID into the public lease field merely to simplify cleanup. Public correlation and private destructive authority are separate contracts.

## Verification contract

The current candidate exact must demonstrate on one unchanged head that:

- existing request/receipt/identity-race suites remain GREEN with explicit effective attachment evidence;
- acquired-ID owner RED returns `sandbox_network_binding` on a foreign effective attachment and performs exact-ID/non-force cleanup;
- binding-owner cases reject different, missing, and additional effective attachments without reaching readiness;
- partial container-create/start/verification/readiness failures retain exact admitted network cleanup authority;
- initial identity-admission failure performs no generated-name removal;
- repository formatting, locked full workspace tests, Clippy/rustdoc, owned-production coverage, hosted negative runtime evidence, and the dedicated positive SELinux lane are separately evaluated on that same exact.

If that exact reaches the create→first-inspect replacement witness next, that failure remains an expected causal RED and must be repaired at the creation-authority boundary rather than by weakening the attachment checks.

## Exact-head linkage

- Network-lifecycle successor: PR #127.
- Executed attachment RED exact: `82afbd1366aa7076c97470f3626686b614c8d29b`.
- Executed CI: run `35636592189`; verify job `106455412479`; hosted rootless/AppArmor negative was GREEN.
- Attachment/partial-cleanup production repair: `6629fd30fa7c011f5aaff1b19ec1b116db2696bd`.
- Positive application fixture repair: `43df86fc8d2e6493d14c90bf04080a0349eb9b91`.
- Identity-race fixture repair: `463e6d9cc465f638f04c3e3cbe0e1b286b246838`.
- Fixed-width entropy boundary restoration/current code exact before this document commit: `580ff3d70c1da22707a1f172527878fa395a11a3`.
- Dependent post-admission rebind RED: PR #142 exact `3aa1425be987851a3c151402bd3efe9edf244323`.
- Durable no-admitted-ID recovery: issue #141.
- Related hostile tests:
  - `tests/podman_application_service_network_acquired_id_owner_red.rs`
  - `tests/podman_application_service_network_binding_owner_red.rs`
  - `tests/podman_application_service_network_create_inspect_toctou_red.rs`
  - `tests/podman_application_service_network_identity_failure_cleanup_owner_red.rs`
  - `tests/podman_application_service_network_identity_provenance_owner_red.rs`
  - `tests/podman_application_service_network_partial_cleanup_owner_red.rs`
  - `tests/podman_application_service_network_termination_owner_red.rs`

## References

The Podman Project. (2026). *Container inspection data structures* [Source code]. GitHub. https://github.com/containers/podman/blob/main/libpod/define/container_inspect.go

The Podman Project. (2026). *podman-inspect — Display artifact, container, image, volume, network, or pod configuration* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-inspect.1.html

The Podman Project. (2026). *podman-network-create — Create a Podman network* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-create.1.html

The Podman Project. (2026). *podman-network-inspect — Display the network configuration for one or more networks* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

The Podman Project. (2026). *podman-network-rm — Remove one or more networks* [Documentation]. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html
