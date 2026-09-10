# Command hold-gate binding traceability

Status: active Draft evidence for issue #25 and Proposed ADR-0008. This document does not authorize release.

## Problem and current code truth

`RootlessPodmanAdapter::run_command_at` still serializes the consumer-requested argv into Podman's OCI `--entrypoint`, creates and initializes that container, starts it, and only then samples live effective seccomp, capability and LSM evidence. The consumer is therefore the OCI initial process. Once `podman start` succeeds, cleanup after a failed attestation cannot undo code execution that was already possible.

The repository now contains the prerequisites for a stronger boundary, but they are not yet composed into `run_command_at`:

- `qsr_runtime_gate` blocks on a runtime-generated one-time token, emits `QSR_GATE_RELEASED` only after exact token acceptance, flushes that trusted acknowledgement, detaches consumer stdin, and then `exec`s the exact consumer argv;
- `RuntimeGateArtifact` stages only an independently expected SHA-256/architecture-matching self-contained ELF64 ET_EXEC/ET_DYN gate, requires a bounded valid program-header table and at least one `PT_LOAD`, and rejects `PT_INTERP` as an unsafe loading boundary;
- `RuntimeGatePodmanAdapter::plan_command_binding` keeps stdin open, mounts that verified artifact read-only as `/qsr-runtime-gate`, selects it as OCI entrypoint, and carries the exact consumer argv behind a fresh 256-bit release token;
- `RuntimeGatePodmanAdapter::release_command_gate` addresses only an admitted exact 64-lowerhex container ID, uses bounded `podman attach --sig-proxy=false`, consumes the token once, requires the trusted acknowledgement, closes the control stdin, and reaps only the local attach client.

These components are prerequisite capability, not containment proof. The canonical production path must own one lifecycle from artifact verification through cleanup.

## Causal evidence ledger

### Pre-attestation execution

Exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`, verify `102428599037` and branch coverage `102428599228` executed `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation`. The hostile fixture observed a consumer side effect after `podman start` and before effective-process attestation failed. This is the controlling P0 counterexample.

### Earlier init/configuration hold

Test-bearing `9e36766abc0a553f45c62fa327b34f1315dc0db9`, executed on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / CI `34353894509`, proved production did not invoke `podman init`: unavailable init and invalid configured isolation both reached start/payload execution. Minimum production `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` plus formatter-only `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e` introduced `create -> exact ID -> init -> pre-start configured-state contradiction checks -> start -> live process verification`. This rejects disproven configuration earlier but deliberately does not relabel static configuration as effective process evidence.

### Real held-gate capability and packaging

Exact `7da7d3b36ec8830ab16d776e5ed1a93f7a16b8ad`, CI `34379177369`, hosted negative rootless/AppArmor job `102559489017` demonstrated on Ubuntu 24.04/rootless Podman that a static runtime-owned gate can remain the only running process, expose effective seccomp/capability/ambient/LSM evidence, preserve its digest, and `exec` the exact consumer argv after explicit release.

Packaging RED `304d63c2722a35b9f05f8466bbea81492a1533a6`, CI `34381329199`, verify `102566637062` proved the Cargo package exposed no gate. Minimum production `3bd5d4319aba7c23982bbd18c77d424c25c3a510` added `qsr_runtime_gate`; CI `34381498621` made the packaging regression GREEN before the unchanged pre-attestation RED. Subsequent E2E uses the production gate source rather than a duplicate test-only implementation.

### Gate binding and stdin liveness

Gate-binding exact `5a591a582ca955ccd24b8666a779de279062fae7`, CI `34393466321`, verify `102607270660` failed for the intended cause: observed create argv still used consumer `--entrypoint=[...]` and did not deliver `/qsr-runtime-gate`.

`RuntimeGateArtifact` and `RuntimeGatePodmanAdapter::plan_command_binding` then modeled immutable gate delivery and one-time release data. Stdin-liveness RED `0ec969b70d647eef9c69149875070136648b21e7`, CI `34402406692`, verify `102637216130` failed because the create fragment lacked `--interactive`; minimum production `b35fce53527c872abd00cffb34b0b831cfa7af74` added only that requirement. Exact `d6a3078e5b9b1983b72101aacfb4111b110fc501`, CI `34402589899`, verify `102638265681` made that focused regression GREEN.

### Bounded release and trusted acknowledgement

The release-control lineage added exact-ID `podman attach --sig-proxy=false`, one-time token consumption, bounded acknowledgement parsing, attach-client timeout/kill/reap behavior, and consumer-stdin separation. A later real-gate regression exposed a protocol contradiction: the controller required `QSR_GATE_RELEASED`, but the trusted gate originally proceeded directly from token validation to consumer `exec`. The gate now emits and flushes `QSR_GATE_RELEASED\n` after exact token equality and before `exec`; failure to write/flush is fail-closed. Focused gate ACK/stdin regressions pass in the later command-runtime lineage before the inherited hold-gate integration RED.

### Runtime-gate loader boundary

Issue/PR #109 proved digest and ELF machine identity were insufficient if an admitted gate contained `PT_INTERP`: image-controlled interpreter resolution could execute before trusted gate `main`. Exact predecessor `cbd43c9378b64f8fd678b478439b9214ac7515ff`, CI `34436990992`, verify `102744034365` made all five loader-admission cases GREEN: self-contained ET_EXEC/ET_DYN positive controls, `PT_INTERP` rejection, truncated program-header rejection, and overflowing `e_phoff` rejection. Gate packaging/binding/release/ACK/stdin also passed before inherited #25. Docs/rustdoc descendant `866acee2c1a3dca36ca2b028cb64c5229783daeb` was a strict descendant of canonical #14 and was normally fast-forward adopted; GitHub records #109 merged into the canonical development ancestry at that exact SHA. This is not protected-branch or release acceptance.

### Current canonical source

After #109 adoption, test-only `01e023b60c0f2cf5c3687c784029484bcfe86907` copied the valid #105/#106 malformed-successful-create ownership regressions onto current ancestry without importing stale child production/workflow state. Those tests address a separate exact-ID cleanup authority defect and do not weaken or replace #25. The production `run_command_at` at that head still selects the hostile consumer as OCI entrypoint, so the original P0 remains open.

No predecessor GREEN transfers to a moved head. Every current integrated candidate must reacquire full tests, owned-production coverage, review/security gates, real rootless evidence and dedicated positive effective-LSM evidence.

## Required production lifecycle

The selected boundary is:

```text
verify immutable gate artifact
  -> create trusted gate as OCI PID 1 with exact consumer argv held behind token
  -> acquire exact container ID
  -> initialize without running consumer
  -> reject pre-start configuration contradictions
  -> start trusted gate only
  -> sample live effective seccomp/capability/LSM evidence from that running gate process
  -> make positive policy decision
  -> bounded one-time exact-ID release
  -> require trusted pre-exec acknowledgement
  -> keep consumer stdin detached
  -> gate execve(exact consumer argv)
  -> bounded wait/log collection
  -> exact-ID cleanup
```

EOF, wrong token, duplicate/late release, attach spawn/write/read/wait failure, malformed or missing acknowledgement, timeout/cancellation, missing effective evidence, or cleanup uncertainty must not authorize consumer execution. Generated `qsr-cmd-*` names are correlation/result metadata only after a concrete ID can exist; they are never a fallback destructive authority.

## Why start cannot precede the gate

OCI Runtime Specification v1.3.0 defines `create` so the user-specified process must not yet run, and defines `start` as the operation that runs the configured process. Therefore putting hostile consumer argv in OCI `process.args`/Podman entrypoint and attempting effective-process attestation only after `start` creates the exact ordering defect demonstrated by the hostile side-effect regression. The runtime-owned gate changes the configured process itself: `start` runs the trusted gate, not the consumer; effective evidence can then be sampled while the consumer remains blocked.

Podman's current create/start documentation is consistent with the control-channel prerequisite: an interactive container keeps stdin available; when detached, reads block until later attachment. Podman also warns that stdin may be consumed as soon as input becomes available, which is why host-side write success is not release proof. Release authority requires exact token validation plus a trusted gate acknowledgement, not merely an open interactive session.

## Invariants retained during integration

- exact requested argv semantics and argument boundaries; no shell;
- immutable digest-pinned/no-pull hostile image identity;
- runtime-owned independently expected gate digest and architecture, self-contained ELF loading boundary;
- exact acquired container ID as lifecycle/destructive authority;
- rootless execution, read-only rootfs, no-new-privileges, all capabilities dropped, private namespaces, deny-by-default network, non-root identity, CPU/RAM/PID/tmpfs/lease bounds;
- source artifact read-only/noexec/nosuid/nodev staging when present;
- configured-state checks remain distinct from effective-process evidence;
- bounded output/wait semantics and cleanup-error precedence;
- no retry/sleep masking, static-only attestation fallback, hostile-image attestor, name rebinding, reusable consumer-facing interactive session, or generated-name cleanup fallback.

## Remaining source work

The release primitive, trusted acknowledgement and loader admission are no longer the missing pieces. The next causal implementation slice is composition: `RootlessPodmanAdapter::run_command_at` must consume a release-authorized `RuntimeGateArtifact`/gate binding rather than constructing the consumer as OCI entrypoint. The current gate-binding RED and original hostile side-effect RED must both be rerun against that integrated implementation. The public/CLI construction path must also receive immutable gate identity without embedding a mutable sibling path or trusting a gate found inside the hostile image.

A separate current-lineage RED candidate for #105/#106 proves malformed successful-create stdout must use a valid runtime-owned cidfile exact ID for cleanup and must perform no generated-name destructive cleanup when no admitted ID exists. Resolve that ownership defect without allowing it to distract from or weaken the P0 hold/attest/release lifecycle.

## References

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification, version 1.3.0: Runtime and lifecycle*. https://specs.opencontainers.org/runtime-spec/runtime/

Open Container Initiative. (2025). *Open Container Initiative Runtime Specification, version 1.3.0: Configuration*. https://specs.opencontainers.org/runtime-spec/config/

Podman Authors. (2026). *podman-create — Create a new container*. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman Authors. (2026). *podman-start — Start one or more containers*. https://docs.podman.io/en/latest/markdown/podman-start.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application Container Security Guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
