# QSR immutable central CI caller candidate

Status: implemented caller candidate; not activated, merged or released.

This private candidate is reconstructed from PR #153 head
`eda024696baa72b8a3b7c56d4832c459cac63219`, not published.
It transplants the complete PR #154 caller path delta and pins central
producer PR #2601 at its actual `41c7ab5a3c19065143752d9d42f93ec72a41fcf4`
source commit. PR #153 source ancestry and its other product tests are
otherwise untouched. Applying this patch creates a new unreviewed head.

The only product workflow retains every pull_request and protected develop push
trigger, read-only contents permissions and PR-scoped cancellation. Its concurrency
prefix `qsr-caller-` differs from the producer's `qsr-central-` to avoid cancelling
its own reusable execution. It declares one job, `ci`, with no runner, step, input,
environment, secret inheritance or optional skip override.

It pins the actually published central commit
`41c7ab5a3c19065143752d9d42f93ec72a41fcf4`, central Draft PR #2601.
The producer pin identifies the PR #2601 local Git object and observed
GraphQL PR head; its exact workflow bytes are hashed in the adjacent
scratch manifest. This record does not claim an independent remote Blob
API retrieval or complete producer approval. The three-file caller-test
enforcement delta from `0dbffa4214f9e3e5ac0f6896bbc338a280cb94fb` to this pin
has a retained independent source verdict; full producer execution, source/evidence
custody repair, runner acceptance and formal App approval remain separate.
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
Draft bypass/SUCCESS is not APPROVED. No author self-approval, fabricated verdict, runner ACL expansion, CI retry,
merge or provider credential transfer occurred in this scratch preflight.
