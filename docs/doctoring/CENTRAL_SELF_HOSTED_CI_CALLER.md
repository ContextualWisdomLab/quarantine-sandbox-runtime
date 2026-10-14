# QSR immutable central CI caller candidate

Status: source caller published on PR #1; dedicated runner activation and release not proved.

The product caller was published on PR #1 and retains the reviewed caller
profile. This successor pins merged central producer PR #2601 at its actual `2757ca42867b0f4fc719f8d0c3c755f5507c4e18`
source commit. This pin update creates a new exact product head; predecessor
checks and approvals do not transfer.

The only product workflow retains every pull_request and protected develop push
trigger, read-only contents permissions and PR-scoped cancellation. Its concurrency
prefix `qsr-caller-` differs from the producer's `qsr-central-` to avoid cancelling
its own reusable execution. It declares one job, `ci`, with no runner, step, input,
environment, secret inheritance or optional skip override.

It pins the actually published central commit
`2757ca42867b0f4fc719f8d0c3c755f5507c4e18`, central PR #2601, merged into main at `6597e37ff677d12960fa0210c9fbf604728ffa92`.
The producer workflow blob matches that merged commit and the observed main.
The successor adds an `x64` runner label and an exact one-test positive LSM
output contract. The product has the referenced scripts and named ignored test.
Runner admission, actual product Actions and formal App approval remain separate.
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
The central review route and consumer application approval are independent.
The requested Noema/OpenCode model remains literal `auto` through personal
LiteLLM; a historical central review does not approve this product head.
Draft bypass/SUCCESS is not APPROVED. No author self-approval, fabricated verdict, runner ACL expansion, CI retry,
merge or provider credential transfer is implied by this source update.
