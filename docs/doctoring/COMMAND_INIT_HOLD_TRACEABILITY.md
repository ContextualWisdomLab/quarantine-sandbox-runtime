# Command init-hold capability traceability

Status: executed RED; focused production GREEN not yet claimed.

## Problem and exact evidence

Issue #25 already proved that the command-execution path can execute hostile payload code before effective process attestation: production creates the consumer entrypoint, invokes `podman start`, and only afterwards samples live process security evidence. Cleanup cannot undo code that has already run.

The first backend-capability counterexample was checked in at `9e36766abc0a553f45c62fa327b34f1315dc0db9`. An unrelated mutable fake-process inode race prevented that test from reaching its intended cause at `52b85226ba9fcce46da02830160dfb010dcaefa6`. Commit `38e97bf9f2c9969a82ed035899946ec43647952e` repaired only that fixture race by keeping the application-service fake executable immutable.

Native CI run `34350868630`, verify job `102463543925`, then executed both intended counterexamples after the existing command-runtime and application-service suites had passed:

- `unavailable_init_capability_fails_closed_without_releasing_payload`: observed `info -> create -> start -> container inspect -> rm`; the payload side-effect marker existed. A failed or absent init capability therefore does not currently stop release because the adapter never calls `podman init`.
- `invalid_prestart_evidence_is_rejected_after_init_without_releasing_payload`: observed the same premature `start`, and the payload marker again existed before the deliberately invalid read-only-rootfs evidence was rejected.

The negative rootless/AppArmor lane `102463543911` passed on that exact head. Dedicated positive-LSM job `102463543920` had no result at the evidence sweep and is not acceptance evidence.

Commit `d36a51bbc4769754dcc777c31fcb31bb68850aca` updates only the checked-in immutable fake-Podman command modes so the existing test infrastructure can represent the new `init <acquired-container-id>` operation without creating another mutable-executable race. It is fixture preparation, not production remediation and not a GREEN claim.

## Decision boundary

The next minimum production slice is deliberately narrower than the complete issue #25 solution:

1. after a trustworthy acquired container ID exists, invoke `podman init <id>`;
2. if init is unavailable or fails, clean up by that acquired ID and fail closed without `start`;
3. while the container remains initialized/not-started, reject static/configuration evidence that already disproves required controls;
4. only then may the existing start path proceed to the current live effective-process verification.

This slice is useful because Podman documents `init` as performing the work needed for start without starting the container, and OCI defines `created` as a state in which the container process has not executed the user-specified program. It creates a real pre-execution rejection point. It does **not** prove final effective seccomp/LSM/capability state: controls such as AppArmor on-exec state and seccomp installation can be finalized near `execve`. Therefore static `container inspect` after init must not be relabeled as effective attestation, and the older payload-side-effect RED remains the acceptance test for the subsequent hold/attest/release slice.

Rejected alternatives remain: merely moving existing static inspect before start; trusting a hostile image to supply an attestor; plain `podman exec`; retry/sleep/mutex masking; and treating cleanup after premature execution as equivalent to prevention.

## Owner and release consequence

`quarantine-sandbox-runtime` remains the canonical owner. The implementation belongs in the rootless-Podman infrastructure ACL while the application/domain contract remains provider-neutral and fail-closed. No consumer repository should copy this runtime source or depend on a mutable branch.

No merge, version, tag, package, GitHub Release, SBOM/provenance publication, or consumer version bump is authorized until an unchanged integrated head has full Rust tests/Clippy/rustdoc/owned 100% coverage, the complete pre-execution attestation P0 is GREEN, real positive-LSM evidence exists, and protected-branch review/security gates are satisfied.

## References

Open Container Initiative. (2026). *Open Container Initiative runtime specification: Runtime and lifecycle*. GitHub. https://github.com/opencontainers/runtime-spec/blob/main/runtime.md

Podman. (2026). *podman-init — Initialize one or more containers*. Podman documentation. https://docs.podman.io/en/v5.2.5/markdown/podman-init.1.html

Open Container Initiative. (2026). *runc*. GitHub. https://github.com/opencontainers/runc

Open Container Initiative. (2026, July 16). *Add an optional pre-exec wait FIFO to runc exec* (Issue #5373). GitHub. https://github.com/opencontainers/runc/issues/5373
