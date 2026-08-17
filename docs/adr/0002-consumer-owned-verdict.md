# ADR 0002: Consumer-owned verdict

- Status: Accepted
- Date: 2026-08-17

## Context

Execution completeness and maliciousness are different. Static-only analysis may complete while still lacking evidence needed for a safe verdict.

## Decision

The runtime returns `completed`, `inconclusive`, or `failed` as execution dispositions and always sets `consumer_verdict_required=true`. It exposes no malicious/benign verdict field.

## Consequences

- Wardnet can combine runtime evidence with context, threat intelligence, policy, and human review.
- Other consumers can apply different quarantine or rejection policies.
- `completed` cannot be misrepresented as `benign`.
