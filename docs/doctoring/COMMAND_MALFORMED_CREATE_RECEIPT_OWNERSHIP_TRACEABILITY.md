# Command malformed-create receipt ownership traceability

Last reviewed: 2026-09-09 KST

## Authority and scope

This record belongs to the `sandbox_execution` Core bounded context and its `infrastructure::podman` adapter. It is the doctoring record for issue #105 and the test-bearing descendant of Draft #14 exact parent `bc9450998848dbb690a74aec7b11878f80b4da62`.

The affected boundary is lifecycle/destructive authority after `podman create` has succeeded but its stdout cannot be admitted as a long container identifier. It does not change `application_service`, `artifact_analysis`, consumer authorization, provider/model semantics, or any public wire schema.

## Problem

Draft #14 already creates a runtime-owned temporary receipt path and passes it through `--cidfile=<path>`. On a nonzero create result the runtime reads that receipt and, when it contains a valid long identifier, cleans up by acquired ID. On a successful create, however, the current implementation parses only stdout. If stdout is malformed or empty, it calls the malformed-create cleanup path with the generated `qsr-cmd-*` name.

That fallback violates the lifecycle ownership decision established by issue #36. A generated name is correlation/audit metadata; after the runtime has acquired a concrete container identity it must not re-resolve a mutable name to select a destructive target. The malformed-success path is especially important because `--cidfile` may already contain the exact resource identity even when stdout is unusable.

Podman's current `podman create` documentation states that the created container ID is printed to stdout and separately documents `--cidfile=file` as writing the container ID to a file. The two outputs therefore provide distinct observation channels for the same created resource; this runtime already chooses the cidfile location itself and can treat an admitted value from that runtime-owned receipt as lifecycle evidence rather than falling back to a mutable name.

NIST SP 800-190 is broader authority rather than a direct prescription for this exact CLI edge case. It frames container deployment and management as a security-relevant lifecycle and recommends addressing container-platform security concerns across that lifecycle. Here it supports retaining a fail-closed, auditable resource-ownership boundary rather than weakening cleanup selection when one observation channel is malformed.

## Checked-in RED

Test-bearing commit `f6b68d810df44780b273289d1d77590440a4937c` adds `tests/podman_command_execution_malformed_create_receipt_ownership_red.rs` without production changes.

The fake Podman contract is intentionally asymmetric:

- `info --format` returns valid rootless/seccomp/AppArmor evidence;
- successful `create` writes a valid 64-hex acquired ID to the supplied runtime-owned cidfile but emits malformed stdout;
- `rm --force <acquired-id>` succeeds;
- `rm --force <generated-name>` fails, modelling a same-principal name replacement or otherwise invalid destructive selection.

The expected future result is the original `MalformedIsolationInspection { operation: "container_create" }` after successful acquired-ID cleanup, with the call log proving that no destructive removal targeted `qsr-cmd-*`.

Until exact CI executes this test past repository/format prerequisites, this is a checked-in RED, not causal RED evidence.

## Alternatives considered

### Keep generated-name cleanup after malformed stdout — rejected

This reopens the exact name-rebinding authority issue repaired for the normal post-create path in #36. Probability of collision does not turn a mutable lookup key into immutable ownership evidence.

### Accept or normalize malformed stdout — rejected

Malformed create output is evidence failure and must remain a typed failure. Truncation, permissive parsing, short-ID admission, or normalization would widen the destructive authority surface and hide a backend-contract violation.

### Retry `podman create` or cleanup — rejected

Retry creates a second lifecycle event and can make ownership less clear. It also treats an identity/evidence defect as transient availability. The existing runtime-owned cidfile should be consumed first.

### Treat absence of both valid stdout and valid cidfile as permission to clean by name — rejected

A generated name remains correlation metadata. If no acquired identity can be admitted, the runtime cannot prove which concrete resource a destructive lookup would select. That state must fail closed and remain explicit leak-risk evidence rather than silently converting the name into ownership authority.

## Minimum causal GREEN after executed RED

If exact CI confirms the intended failure, the smallest repair is confined to the successful-create malformed-stdout branch:

1. read the runtime-owned cidfile using the existing bounded identifier admission path;
2. when it yields a valid acquired long ID, perform cleanup exclusively against that ID;
3. preserve the original malformed-create error if cleanup succeeds;
4. surface cleanup failure if acquired-ID cleanup cannot be proven;
5. never select generated `qsr-cmd-*` as destructive authority after successful create.

No public enum, schema, application-service behavior, analyzer behavior, retry policy, timeout, or provider-specific vocabulary needs to change.

## Verification and release gates

After the causal repair, the unchanged exact head must pass repository validation, rustfmt, full workspace tests, Clippy, rustdoc, complete owned-production statement/function/region/branch coverage, qualifying review/security gates, hosted negative confinement and dedicated positive effective-LSM evidence. Real rootless runtime evidence remains required for any release claim about actual isolation. Only protected integration followed by immutable version/package/tag/release, SBOM, provenance, reproducibility and rollback evidence can become consumer authority.

## References

Podman. (n.d.). *podman-create — Podman documentation*. Retrieved September 9, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
