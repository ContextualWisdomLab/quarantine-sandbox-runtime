# QSR immutable central CI caller candidate

Status: implemented caller candidate; not activated, merged or released.

This branch is based on QSR #151 at `bfa21759db9b9047135634d7ec9b46660d3bd9dc`.
It does not automatically include QSR #150 static-analysis changes. Adoption on
other product heads requires explicit reviewed ancestry and fresh verification.

The only product workflow retains every pull_request and protected develop push
trigger, read-only contents permissions and PR-scoped cancellation. Its concurrency
prefix `qsr-caller-` differs from the producer's `qsr-central-` to avoid cancelling
its own reusable execution. It declares one job, `ci`, with no runner, step, input,
environment, secret inheritance or optional skip override.

It pins the actually published central commit
`91f0ccc23fa19b82ce797573155bb6f6a0f64b5f`, central Draft PR #2601.
Remote immutable workflow bytes were independently retrieved through GitHub's
Blob API and matched SHA-256
`5a9e1006cfc55b0f7c873caa40fdf2defe764eb9370593ccd742a2e7b5b60884`.
Publication is not protected-main integration or real runner acceptance.

The pinned producer owns exact candidate checkout with persist-credentials false,
admission rejection, verify, coverage, branch-coverage, positive-LSM acceptance
and a terminal acceptance job. The product caller cannot grant runner capacity.
The dedicated `CWL QSR hostile workload` group remains a requested, unprovisioned
selector. Existing control/MCP runners are not a substitute. Product source runs
only after producer admission, but these source checks are not an operator-bound
host isolation or prelease attestation.

Prospective check names, not observed remote check-run contexts:
- `ci / admission`
- `ci / verify`
- `ci / coverage`
- `ci / branch-coverage`
- `ci / podman-e2e-positive-lsm`
- `ci / acceptance`

The terminal result requires all five dependencies to succeed. Skipped, cancelled,
failed or missing results are nonpassing. Branch protection/ruleset mapping must
be verified by the authorized owner before treating these names as replacements.
No required check or branch protection setting was modified in this work.

Hosted-only `podman-e2e-negative-rootless-apparmor` is DISABLED/NOT_RUN in this
candidate. Its unavailable-effective-LSM rejection/cleanup evidence is missing,
not replaced by static tests or positive SELinux. All retained product coverage,
warning and isolation assertions stay in the immutable producer. Disabling hosted
execution does not justify a release claim.

The product-owned Rust regression no longer requires inline hosted job/checkout
implementation. Instead it checks the immutable central call, absence of secret
inheritance/execution overrides, and unchanged triggers/concurrency. Producer
checkout, tool pins, cleanup errors and fail-closed result semantics are covered
by the pinned central workflow contracts; caller structure tests alone cannot
establish those implementation properties.

AI review is a separate central responsibility under existing .github#2560.
Current source observations still show the legacy free-sidecar model route;
`LLM_GATEWAY_MODEL=auto` metadata alone is not effective deployment. The direct user
requires actual self-hosted Noema/OpenCode model `auto` through personal LiteLLM,
with author-distinct App review publication after substantive independent review.
Draft bypass/SUCCESS is not APPROVED. No author self-approval, fabricated verdict,
runner ACL expansion, CI retry, merge or provider credential transfer occurred
in this caller implementation.
