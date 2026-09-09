# Command malformed-create receipt ownership traceability

Last reviewed: 2026-09-09 KST

## Authority and scope

This record belongs to the `sandbox_execution` Core bounded context and its `infrastructure::podman` adapter. It is the doctoring record for issue #105 and the test-bearing descendant of Draft #14 exact parent `bc9450998848dbb690a74aec7b11878f80b4da62`.

The affected boundary is lifecycle/destructive authority after `podman create` has succeeded but its stdout cannot be admitted as a long container identifier. It does not change `application_service`, `artifact_analysis`, consumer authorization, provider/model semantics, or any public wire schema.

## Problem

Draft #14 already creates a runtime-owned temporary receipt path and passes it through `--cidfile=<path>`. On a nonzero create result the runtime reads that receipt and, when it contains a valid long identifier, cleans up by acquired ID. Before #105's repair, successful create parsed only stdout. If stdout was malformed or empty, it called cleanup with the generated `qsr-cmd-*` name.

That fallback violated the lifecycle ownership decision established by issue #36. A generated name is correlation/audit metadata; after the runtime has acquired a concrete container identity it must not re-resolve a mutable name to select a destructive target. The malformed-success path is especially important because `--cidfile` may already contain the exact resource identity even when stdout is unusable.

Podman's current `podman create` documentation states that the created container ID is printed to stdout and separately documents `--cidfile=file` as writing the container ID to a file. The two outputs therefore provide distinct observation channels for the same created resource; this runtime chooses the cidfile location itself and can treat an admitted value from that runtime-owned receipt as lifecycle evidence rather than falling back to a mutable name.

NIST SP 800-190 is broader authority rather than a direct prescription for this exact CLI edge case. It frames container deployment and management as a security-relevant lifecycle and recommends addressing container-platform security concerns across that lifecycle. Here it supports retaining a fail-closed, auditable resource-ownership boundary rather than weakening cleanup selection when one observation channel is malformed.

## Executed causal RED

Test-bearing commit `f6b68d810df44780b273289d1d77590440a4937c` adds `tests/podman_command_execution_malformed_create_receipt_ownership_red.rs` without production changes.

The fake Podman contract is intentionally asymmetric:

- `info --format` returns valid rootless/seccomp/AppArmor evidence;
- successful `create` writes a valid 64-hex acquired ID to the supplied runtime-owned cidfile but emits malformed stdout;
- `rm --force <acquired-id>` succeeds;
- cleanup against a generated `qsr-cmd-*` name fails, modelling a same-principal name replacement or otherwise invalid destructive selection.

Exact head `054a0e8fc6c746398295703dfd71a138efe404c5` reached this regression in branch-coverage job `102256345545`. The assertion expected the original `MalformedIsolationInspection { operation: "container_create" }` after acquired-ID cleanup; production instead returned `CleanupFailed`. That is the intended causal RED: the malformed-success branch selected generated-name cleanup despite a valid runtime-owned cidfile receipt.

The same workflow run's ordinary verify and complete-coverage lanes encountered the separately tracked #71/#72 `backend_security_info` process/backend instability before or elsewhere in the broad suite. Those inherited failures do not replace or invalidate the dedicated causal RED above, because branch coverage executed this exact regression and exposed the ownership defect directly.

## Alternatives considered

### Keep generated-name cleanup after malformed stdout — rejected

This reopens the exact name-rebinding authority issue repaired for the normal post-create path in #36. Probability of collision does not turn a mutable lookup key into immutable ownership evidence.

### Accept or normalize malformed stdout — rejected

Malformed create output is evidence failure and must remain a typed failure. Truncation, permissive parsing, short-ID admission, or normalization would widen the destructive authority surface and hide a backend-contract violation.

### Retry `podman create` or cleanup — rejected

Retry creates a second lifecycle event and can make ownership less clear. It also treats an identity/evidence defect as transient availability. The existing runtime-owned cidfile should be consumed first.

### Treat absence of both valid stdout and valid cidfile as permission to clean by name — rejected

A generated name remains correlation metadata. If no acquired identity can be admitted, the runtime cannot prove which concrete resource a destructive lookup would select. That state must fail closed rather than silently converting the name into ownership authority.

## Minimum causal repair

Production commit `4f7a670fcec0663ef13a04c6e2ea42e4505df86a` applies the minimum repair to the successful-create malformed-stdout branch:

1. preserve the original `MalformedIsolationInspection { operation: "container_create" }`;
2. read the existing runtime-owned cidfile through `read_command_create_receipt`, which admits only an exact 64-character lowercase hexadecimal container ID;
3. when a valid acquired ID exists, cleanup exclusively through `cleanup_owned_command_container_or_report(&container_id, original)`;
4. if the receipt is absent, return the original typed evidence failure without performing destructive name lookup;
5. if the receipt itself is unreadable or malformed, surface its typed receipt-evidence failure;
6. remove the command-specific generated-name cleanup helpers so the unsafe fallback cannot be selected accidentally.

Commit `cb2c046c6d2d36accaa003996176a6576763d114` updates the pre-existing malformed-success/no-receipt regression so it proves that `--cidfile` is provisioned and that no `rm --force <generated-name>` call occurs when neither stdout nor cidfile provides trustworthy ownership evidence. This complements the #105 regression, which proves successful cleanup by the admitted acquired ID when the receipt exists.

No public enum, schema, application-service behavior, analyzer behavior, retry policy, timeout, or provider-specific vocabulary changes.

## Exact-head stale-contract RED and repair

Commit `32a9f05b1a0f41ad75efff0e3ec8ccf7b1590210` first adopted the current root CI checkout hardening without changing runtime semantics. Native verify job `102286539612` proved the adoption itself was active (`actions/checkout` logged `persist-credentials: false`) and then reached a deterministic stale test contract: `tests/podman_command_execution_malformed_create_cleanup_red.rs::cleanup_failure_is_not_hidden_behind_malformed_container_identifier` expected `CleanupFailed`, while production correctly returned `MalformedIsolationInspection { operation: "container_create" }`.

That old regression encoded the retired generated-name cleanup rule: its fake `create` emitted malformed stdout, never wrote the runtime-owned cidfile, then made `rm --force` fail. Under the #105 ownership model there is no admitted concrete container identity in that fixture, so attempting name-based removal would itself be the defect. The correct invariant is therefore to preserve the malformed-create error and prove that no destructive `rm --force <generated-name>` occurs.

Test-only commit `6bd7fec4299c888d87b3476cc6c0b64851aa1b79` repairs that stale contract. It keeps malformed successful stdout and an absent cidfile, requires the create invocation to contain `--cidfile=`, expects the original malformed-create typed failure, and asserts that no `rm --force` call occurs. Production Rust is unchanged. The acquired-ID cleanup case remains independently covered by the #105 regression where the cidfile contains a valid 64-lowerhex ID.

## CI credential adoption

The predecessor command-runtime workflow still let `actions/checkout` persist its token in the local Git configuration. Current root #1 already disables that behavior. Commit `32a9f05b1a0f41ad75efff0e3ec8ccf7b1590210` adopts the canonical root `.github/workflows/ci.yml` blob exactly: five checkout steps now set `persist-credentials: false`, with action SHA, exact-head ref, runner labels, toolchains and job commands unchanged. The verify log confirms the credential file is removed immediately after checkout before repository validation or Rust execution.

## Verification and release gates

`6bd7fec4299c888d87b3476cc6c0b64851aa1b79` is a test-repair head, not merge or release authority. A fresh unchanged exact head containing production `4f7a670f...`, no-receipt coverage, the stale-test repair and credential-free checkout must pass repository validation, rustfmt, full workspace tests, Clippy, rustdoc, complete owned-production statement/function/region/branch coverage, qualifying review/security gates, hosted negative confinement and dedicated positive effective-LSM evidence. Real rootless runtime evidence remains required for any release claim about actual isolation. Only protected integration followed by immutable version/package/tag/release, SBOM, provenance, reproducibility and rollback evidence can become consumer authority.

## References

Podman. (n.d.). *podman-create — Podman documentation*. Retrieved September 9, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190