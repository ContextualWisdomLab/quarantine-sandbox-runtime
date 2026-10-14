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

It pins the central integration commit
`6597e37ff677d12960fa0210c9fbf604728ffa92`. Central PR #2601 merged into
`main` on October 14, 2026 at 01:54:40 KST under the user's infrastructure
exception. Immutable workflow bytes match SHA-256
`db689bbaba070c4ba92bf3963cdb985afab74e65bee753481b10270b83cd30f0`.
This is producer source integration, not caller merge or real runner acceptance.
The producer unconditionally runs both Python caller oracle modules and rejects
a positive-LSM test filter that executes zero tests. SRC01 character rejection
remains intact. SRC03 job-private source/tool/evidence custody is still unfinished.

The pinned producer owns exact candidate checkout with persist-credentials false,
admission rejection, verify, coverage, branch-coverage, positive-LSM acceptance
and a terminal acceptance job. The product caller cannot grant runner capacity.
The dedicated `CWL QSR hostile workload` group remains a requested, unprovisioned
selector. Existing control/MCP runners are not a substitute. Product source runs
only after producer admission, but these source checks are not an operator-bound
host isolation or prelease attestation.

Observed caller check names on predecessor head `173e1f255...`; new-head
execution is not established:
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
A Noema source-only review on predecessor central head `273931765...` recorded
`served_model=auto` and an author-distinct App approval. That review was dismissed
after head movement; it does not approve this caller or the merged producer head.
The direct user requires actual self-hosted Noema/OpenCode model `auto` through
personal LiteLLM, with author-distinct App review publication after substantive
independent review. `LLM_GATEWAY_MODEL=auto` metadata alone is not inference.
Draft bypass/SUCCESS is not APPROVED. This caller change neither expands runner
ACLs nor transfers provider credentials. Actual caller Actions, formal App review,
QSR runtime isolation and release acceptance remain incomplete.
