# Podman option terminator traceability

## Decision boundary

Consumer image references are validated domain data. Podman still parses its own argv according to provider CLI grammar, so a digest-pinned image reference can remain syntactically option-shaped when its repository/name begins with `-`. The infrastructure anti-corruption layer therefore terminates Podman option parsing with one literal `--` immediately before every consumer-controlled image operand.

Rejecting such an otherwise-valid image reference, shell quoting, joining argv into a command string, or sanitizing the repository name is not an equivalent control. The invariant is that consumer data remains byte-for-byte application intent while `infrastructure::podman` owns the provider grammar boundary.

This control is cumulative with issue #25. The delimiter prevents provider argument injection; the runtime-owned hold/attest/release protocol prevents hostile consumer execution before live effective seccomp, capability, and LSM attestation. Neither substitutes for the other.

## DDD ownership

`application_service` and `sandbox_execution` own validated consumer intent and image identity. `infrastructure::podman` owns translation into Podman's CLI grammar. The provider/data delimiter therefore belongs to the Podman adapter rather than to domain validation.

The invariant currently has three canonical owner paths:

| Owner path | Contract | Evidence |
| --- | --- | --- |
| `RootlessPodmanAdapter::plan_at` | application-service create argv places literal `--` immediately before the exact consumer image, then preserves exact command argv | `tests/podman_option_terminator_red.rs` |
| `RuntimeGatePodmanAdapter::plan_command_binding` | gate create argv places runtime-owned mount/entrypoint options first, then literal `--`, exact image, one-time release token, and exact consumer argv | `tests/podman_runtime_gate_option_terminator_red.rs` |
| `RootlessPodmanAdapter::run_command_at` | canonical one-shot create argv places literal `--` immediately before the exact consumer image | `tests/podman_command_execution_option_terminator_red.rs` |

The eventual issue #25 integration may replace the current consumer `--entrypoint=<argv-json>` with the runtime-owned gate entrypoint, but it must retain the same provider/data delimiter immediately before the image.

## Evidence lineage

### Application-service path

PR #91 introduced the original application-service counterexample. Native CI `34202917157`, coverage job `101985631210`, executed `tests/podman_option_terminator_red.rs` and observed an option-owned argument immediately before the consumer image instead of `--`. Minimum production commit `1b1026a92b7d9ef23a709564c1e22aa0421179ff` inserted one literal delimiter immediately before `request.image_reference` in `PodmanLaunchPlan::container_create_args()`.

Canonical PR #14 later adopted that semantic delta rather than importing the stale PR #91 production tree. PR #91 must remain open until any still-valid historical documentation/evidence delta is completely inherited under the verified-successor rule.

### Runtime-gate binding path

`tests/podman_runtime_gate_option_terminator_red.rs` protects `RuntimeGatePodmanAdapter::plan_command_binding`. Exact predecessor native CI `34469921021`, verify job `102847185623`, executed the regression GREEN while the independent issue #25 composition tests remained RED. The binding preserves the sequence `runtime-owned options -> --entrypoint=/qsr-runtime-gate -> -- -> exact consumer image -> one-time release token -> exact consumer argv`.

### Canonical one-shot command path

Fresh review of predecessor `0a0fec0434099991762ac31333b162d2d55a912e` found that `RootlessPodmanAdapter::run_command_at` appended `request.image_reference` directly after the consumer entrypoint without a delimiter.

Test-only `87d0d193158b954da0c5afc1c25fa8437df32a98` added `tests/podman_command_execution_option_terminator_red.rs`. Formatter-only descendant `4229b75861c377f4c28a0855537068fdeada8163` left the semantic assertion unchanged. Native CI `34469921021`, exact verify job `102847185623`, checked out `4229b758...`, passed repository policy and rustfmt, then produced the intended causal RED: the recorded `podman create` argv put `-consumer/tool@sha256:<digest>` directly after `--entrypoint=...` with no intervening `--`.

Minimum production commit `b6cbb1a118de7ca0e5ca1c94b878d45a0be88d1c` changed one production line only, adding `create_args.push("--".to_owned())` immediately before `request.image_reference`. Commit inspection reports one file changed, one line added, no deletion.

Native CI `34470592752`, exact verify job `102849324829`, checked out `b6cbb1...` and executed `command_runtime_terminates_podman_options_before_consumer_image` GREEN. The same no-fail-fast run continued to expose the independent issue #25 hold-gate and pre-attestation REDs, so this is focused exact-head GREEN for the provider/data boundary, not broad PR or release acceptance.

## Invariants and regression policy

A valid Podman create translation for consumer image data must satisfy all of the following:

- exactly one literal `--` terminates provider options immediately before the image operand;
- the image reference after the delimiter is byte-for-byte the validated request value;
- command arguments retain exact argv boundaries and are never shell-joined;
- digest pinning remains mandatory but is not treated as CLI neutralization;
- runtime-owned labels, resource controls, mounts, identity receipts, and the issue #25 hold gate remain before the delimiter as provider options;
- moving to the runtime-owned gate must not reintroduce a direct consumer entrypoint or move the image ahead of the delimiter.

A future provider adapter may implement an equivalent typed API that does not expose command-line option parsing. Until then, this delimiter is part of the Podman anti-corruption-layer contract.

## Remaining acceptance

The three current owner paths now have code/test semantics for the delimiter. Issue #90 stays open until the remaining valid PR #91 historical traceability/evidence is completely inherited and current protected integration evidence is available. Issue #25 remains independently release-blocking until the canonical one-shot runtime composes the already-proven immutable gate, held start, live effective attestation, bounded one-time release, trusted pre-exec acknowledgement, detached consumer stdin, exact argv exec, wait/log capture, and exact-ID cleanup into one production lifecycle.

No version, tag, package, immutable publication, or containment claim follows from the focused delimiter GREEN alone.

## References

MITRE. (2026). *CWE-88: Improper neutralization of argument delimiters in a command ('Argument Injection')*. Common Weakness Enumeration, Version 4.20.

Podman Authors. (2026). *podman-create — Create a new container*. Podman documentation. The CLI synopsis distinguishes provider options from the positional image and command operands: `podman create [options] image [command [arg...]]`.
