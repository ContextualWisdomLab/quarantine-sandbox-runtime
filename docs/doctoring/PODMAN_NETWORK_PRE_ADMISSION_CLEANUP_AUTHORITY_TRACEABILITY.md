# Podman network pre-admission cleanup authority traceability

## Decision status

Proposed RED on top of canonical network owner #127 exact `b81468346a88c37ed920f6417571b9e7cc846b4d`. This slice changes no production Rust, public API, schema, network create semantics, or cleanup implementation. It exists to prove one authority boundary before any GREEN is attempted.

## Problem

The current network owner correctly stopped using generated `qsr-net-*` names as attachment and cleanup selectors. It now obtains a canonical 64-lower-hex candidate ID from one bounded creation-event receipt, then inspects that exact ID for the expected generated correlation name, `internal=true`, and DNS disabled before container creation.

The remaining defect is the transition between **candidate identity** and **admitted destructive authority**. `RootlessPodmanAdapter::acquire_network_id()` currently invokes `cleanup_admitted_network(&network_id)` when `network_identity_inspect` fails, cannot be parsed, reports a noncanonical ID, or contradicts the expected name/internal/DNS state. At those points the exact ID has not yet passed the P0 corroboration used to establish ownership. A rejected candidate is therefore still able to select a destructive `podman network rm <full-id>` operation.

That is unsafe under the same-name/event-history threat already owned by #48/#127/#141. The bounded event query is a discovery and correlation mechanism. If the invocation's own event is absent, rotated, or outside the client-derived time window while a different matching event is visible, P0 contradiction is evidence that the candidate must be rejected. It is not proof that the candidate belongs to this invocation.

## Required invariant

Network lifecycle authority has two distinct states:

- `candidate_network_id`: a canonical backend ID obtained from bounded receipt/discovery evidence and allowed only as an exact inspection selector;
- `admitted_network_id`: an exact ID that has passed the complete creation-bound corroboration contract and may then be used for container binding, post-admission inspection, and exact non-force cleanup.

No `network rm` operation is permitted while the ID is only a candidate. Inspection command failure, malformed inspection, ID mismatch, wrong generated name, `internal=false`, DNS enabled, or any other P0 contradiction returns fail closed and leaves the possible orphan to the durable recovery owner #141. Public correlation, canonical-ID syntax, event membership, label membership, age, or dangling state alone never grants destructive authority.

## Executable RED

`tests/podman_application_service_network_pre_admission_cleanup_authority_red.rs` uses the current parser surface intentionally (`Network` remains uppercase) so this finding is independent of the pending Podman-v6 lowercase-`network` transport RED.

For each of four hostile states—inspection command failure, wrong network name, external network state, and DNS enabled—the fixture requires:

1. rootless/backend prerequisites succeed;
2. `network create` succeeds;
3. the bounded event history nominates one canonical candidate ID under the expected generated correlation;
4. exact-ID P0 inspection fails or contradicts the intended network;
5. launch fails before container creation;
6. the rejected candidate ID is never passed to `podman network rm`;
7. network-level `--force` removal remains forbidden.

Current #127 production is expected to fail requirement 6 because the rejected candidate is removed in each path. That failure is the causal RED. Do not repair the pending JSON field casing, missing capability/DNS evidence, remote clock-domain parity, or durable recovery in order to make this witness GREEN.

## Minimum GREEN direction

The minimum owner-local repair is state separation inside the existing adapter: perform exact-ID P0 corroboration without any destructive cleanup, promote the ID only after all creation-bound predicates pass, and permit cleanup only from the admitted state. Pre-admission failures can leak an owned network, but deleting a possibly foreign network is the higher-severity error; #141 owns restart-safe orphan reconciliation and must recover only after private durable intent plus exact-ID corroboration.

A later private recovery correlation label may strengthen provenance, but a label remains discovery/corroboration input rather than deletion authority. Do not make the RED depend on a future persistence implementation.

## Rejected alternatives

- removing the candidate by generated name, prefix, label, age, dangling status, or event membership;
- treating a syntactically canonical 64-hex ID as ownership proof;
- keeping the current deletion and adding retries/sleeps around event or inspect timing;
- using `podman network rm --force` to hide in-use contradictions;
- weakening name/internal/DNS P0 checks;
- widening the current PR into durable recovery or a new database before the launch-time authority defect is repaired.

## Evidence and references

Current source authority is `src/infrastructure/podman.rs` on #127 exact `b81468346a88c37ed920f6417571b9e7cc846b4d`, especially `acquire_network_id()`, `cleanup_admitted_network()`, and `parse_network_creation_receipt()`. Review `5293564412` records the owner finding. Issue #141 defines the complementary durable-recovery rule: candidate enumeration is not destructive authority and promotion requires canonical full-ID corroboration against private intent and P0 state.

Podman documents `podman network rm <network>` as deletion of the specified network; `--force` additionally removes containers using that network. The RED therefore forbids deletion before ownership admission even though the current command is non-force.

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-rm.1.html

## Completion gate

Keep this RED Draft until it executes on an unchanged exact head for the intended pre-admission removal cause. Only then apply the smallest causal production repair, reacquire the four hostile cases plus current receipt/P0/binding/cleanup controls, full repository/fmt/tests/Clippy/rustdoc, complete applicable owned-production statement/function/region/branch/edge coverage, qualifying security/review/thread gates, positive effective isolation where applicable, dependency-safe parent succession, protected integration, and immutable release evidence.
