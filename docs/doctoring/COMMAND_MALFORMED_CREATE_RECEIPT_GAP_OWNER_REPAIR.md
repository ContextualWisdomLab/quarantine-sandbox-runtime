# Command malformed-create receipt Gap owner repair

Reviewed 2026-09-17 KST against reopened Draft PR #106 exact `e0d035d2ea6fa979db50d710d08dce49d0ad6d36`, current exact base `aca827d7bd45f3df289456176730c781bd6d1164`, canonical command-runtime owner #14, and repository-wide Gap owner #121.

## Reopen and owner-boundary finding

#106 had been closed without merge even though both its own authority text and canonical #14 stated that the #105 baseline/traceability delta had not yet been proven completely succeeded. That closure therefore did not satisfy the repository PR-zero rule. The PR was reopened before modifying source.

Review `5229930222` narrowed the remaining documentary delta. The branch still changed repository-wide `docs/product-technical-gap-baseline.md`, but the stable issue #105 causal history belongs to the command-runtime owner-local record, not to a second live Gap writer. The repair is migration-first: preserve the remaining causal/succession evidence here and in `COMMAND_MALFORMED_CREATE_RECEIPT_OWNERSHIP_TRACEABILITY.md`, then restore only the global Gap file to the current exact-base blob `a5f9aefd41ddac7e021d6813e1af3c7b2dd73e62`.

## Retained issue #105 evidence

Test-bearing `f6b68d810df44780b273289d1d77590440a4937c` models successful `podman create` writing a valid 64-lowerhex acquired container identity to the runtime-owned cidfile while stdout is malformed. Exact RED `054a0e8fc6c746398295703dfd71a138efe404c5`, native CI `34284356731`, branch-coverage job `102256345545`, reached the dedicated regression and failed for the intended cause: production returned `CleanupFailed` because it selected generated-name cleanup, while the contract required the original malformed-create error after cleanup by the acquired identity.

Minimum production `4f7a670fcec0663ef13a04c6e2ea42e4505df86a` changes only the successful-create malformed-stdout branch. It reads the existing runtime-owned cidfile, admits exactly 64 lower-case hexadecimal bytes, cleans exclusively by that acquired identity when present, preserves cleanup-failure precedence, and removes the command-specific generated-name destructive fallback. No generated correlation name becomes backend lifecycle authority.

Test repair `cb2c046c6d2d36accaa003996176a6576763d114` updates the historical no-receipt regression: successful create with malformed stdout but without trustworthy cidfile identity must preserve the malformed-create error and prove no `rm --force <qsr-cmd-*>` operation occurs. `5deb56fc84bac0dd28a5d2d4b54fc73bcaa25fc3` records the causal RED and minimum repair in the main owner TRACEABILITY.

The branch also non-force adopted root checkout credential hardening through `32a9f05b1a0f41ad75efff0e3ec8ccf7b1590210`, then repaired the stale pre-#105 fixture with `6bd7fec4299c888d87b3476cc6c0b64851aa1b79`; formatter-only `e0d035d2ea6fa979db50d710d08dce49d0ad6d36` changed no runtime semantics. Historical broad jobs that later rotated `BackendInvocationFailed { operation: "backend_security_info" }` are process-boundary evidence, not reasons to weaken #105.

## Succession boundary

Canonical #14 already contains the #105 production semantics and focused regressions, but successor completeness is stronger than source similarity. Every valid test, fixture, contract, doctoring decision, and causal evidence reference must be preserved or deliberately superseded before #106 can become PR 0. Removing the stale global Gap delta solves one outstanding owner-boundary defect; it does not itself prove complete succession or authorize closure.

`command_runtime`/Podman infrastructure retains exact acquired-ID lifecycle authority. Generated `qsr-cmd-*` names remain correlation/audit metadata. `docs/product-technical-gap-baseline.md` remains #121's live repository-wide owner path.

## Repair decision and gate

Selected repair: retain source/tests/workflow and both owner-local doctoring records, restore only the global Gap file to the current exact base by ordinary fast-forward, and keep #106 Draft/open until a fresh compare against canonical #14 demonstrates complete succession or a stable parent permits ordinary non-force restack and exact-head revalidation.

Rejected alternatives are simple Close, force rebase, copying #121's latest live ledger into this historical child, deleting causal records, or treating generated names as cleanup fallback.

Any moved exact head must reacquire its own checks; predecessor results do not transfer.
