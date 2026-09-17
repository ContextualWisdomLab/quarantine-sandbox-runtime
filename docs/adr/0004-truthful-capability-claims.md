# ADR 0004: Truthful capability claims

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

Sandbox products are easy to overstate: command construction is not container isolation, a planned gVisor adapter is not implemented isolation, static analysis is not behavioral evidence, and a queued security workflow is not passing evidence.

## Decision

Every capability is classified by evidence rather than aspiration.

- `implemented_on_protected_branch`: code and required evidence are present on the protected branch.
- `implemented_on_active_pr`: code exists on an active PR but is not shipped truth.
- `planned`: accepted design only.
- `research_only`: exploratory work with no production contract.
- `superseded`: replaced by a newer accepted decision.
- `out_of_scope`: intentionally excluded.

Fake-process tests may prove argv/lifecycle behavior but cannot prove real Podman/gVisor/container isolation. Static evidence cannot be described as runtime behavior. Pending/queued/skipped/cancelled/failed/stale checks are non-passing.

## Consequences

README, PRD, architecture, PR bodies, release notes, and buyer-facing claims must distinguish current protected truth from active-PR and future capabilities. Product-gap documentation is not completion; concrete gaps remain executable work.
