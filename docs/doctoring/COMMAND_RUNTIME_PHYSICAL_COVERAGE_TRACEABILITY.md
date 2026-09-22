# Command/runtime physical coverage succession

## Scope

This record narrows the repository-wide coverage finding exposed by command/runtime succession PR #143. It does not change the missing/null Podman inspection contract, application-service domain semantics, or network lifecycle ownership.

## Executed evidence

Exact `fba1118776f1fbd3595ad222022ee9b0ae552201`, native CI `35740227414`, executed the repaired missing-isolation-evidence parser. Exact checkout, dependency locking, repository policy, CI evidence-contract tests, the six focused parser controls, and hosted rootless/AppArmor negative acceptance reached GREEN. The same run produced exact LLVM coverage artifacts and failed the repository's unchanged 100% physical-source admission.

Re-evaluating those artifacts with the canonical physical-source union rule from command/runtime repository-fitness owner #112 leaves these exact deficits:

- `src/infrastructure/podman.rs`: physical source lines 587, 593 and 1867; the only missing physical branch outcome is the true outcome at the post-probe readiness deadline around line 1866.
- `src/application_service/coordinator.rs`: eight region-only gaps around request-validation propagation, registry-lock propagation, post-launch cleanup/state recovery and successful-termination/expiry registry completion.
- `src/infrastructure/podman_runtime_gate_binding.rs`: five region-only gaps around request validation and release-channel error propagation.
- `src/pr_source_artifact.rs`: fourteen region-only gaps around fail-closed host filesystem I/O propagation and staging operations.

The three Podman source-line deficits are not introduced by the missing/null parser repair. Two are command setup error-propagation edges around OS entropy and private receipt-directory creation. The third is the second readiness deadline observation after a failed connection probe. Existing helper tests prove the typed entropy/tempdir taxonomy, but do not execute the same production call-site propagation. Real-clock polling must not be made flaky merely to hit the readiness branch; a later causal repair should expose a deterministic private decision seam or an equivalent bounded clock abstraction.

## Current focused repair

Successor coverage work adds `tests/application_service_coordinator_boundary_coverage.rs` through the public coordinator API. It covers three previously underrepresented boundary outcomes without changing production code:

- an invalid request is rejected before backend launch;
- an identical active idempotency replay returns the registered lease without a second backend launch;
- a different owner cannot turn an otherwise valid lease into backend cleanup authority.

These tests are repository-fitness evidence, not a transfer of `application_service` domain ownership into command/runtime. They intentionally do not reach into the private registry or manufacture poisoned state.

## Remaining action boundary

Do not lower physical-source denominators, exclude error paths, mutate process-global temporary-directory state, rely on scheduler timing, or add retries. Remaining command/runtime Podman edges require deterministic owner-local seams; application-service coordinator state-poison/recovery edges remain subject to the coordinator owner's invariants; source-artifact host-I/O paths must stay fail closed and use real or deterministic host-boundary evidence. Every moved exact must reacquire its own repository policy, format, full tests, Clippy/rustdoc, complete physical statement/function/region/branch/edge coverage, hosted negative runtime evidence, and applicable positive effective-LSM evidence before merge authority exists.
