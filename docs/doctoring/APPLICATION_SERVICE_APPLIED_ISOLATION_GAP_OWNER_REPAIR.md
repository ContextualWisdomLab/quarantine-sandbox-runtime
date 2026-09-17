# Application-service applied-isolation gap owner repair

Reviewed 2026-09-17 KST against Draft PR #19 exact `7eb867c685e205b097e668f2a6fe9c0ab7b3cbed`, exact base `0f765af1a4eea83029febee3b24c55cd7e7ce4e1`, and repository-wide Gap owner PR #121.

## Problem and ownership boundary

PR #19 is a focused application-service RED branch. It owns hostile evidence for backend-applied resource, namespace, immutable-image, and direct-argv/ENTRYPOINT integrity. It does not own the repository-wide live product/technical Gap ledger. Review `5229900011` found that the branch still changed `docs/product-technical-gap-baseline.md`, so a later integration could replay a 2026-09-06 snapshot over the current #121 owner graph.

The repair is migration-first. Stable #19-specific causal/dependency context is retained here and in the existing owner-local TRACEABILITY documents before the global Gap file is restored byte-for-byte to this PR's exact-base blob `5f17a748cf92810963ea67b30ce54675a7c6d919`. This is an ownership repair only; it does not convert a staged RED into executed evidence or authorize production GREEN.

## Retained owner-specific evidence

The original resource-attestation lane in `tests/podman_effective_resource_bounds_red.rs` requires fail-closed cleanup and no lease publication for missing or contradictory `/tmp` hardening, missing `noexec`/`nosuid`/`nodev`, duplicate/conflicting tmpfs options, unexpected writable tmpfs mounts, and missing or mismatched timeout state. Backend inspection can bind configured/applied state, but real release evidence still requires authoritative cgroup-v2, live mount/options/size, and behavioral wall-time termination proof tied to the exact sandbox.

Issue #45 / test-bearing `d9c5b201e1e6548f29de438e7f470445118a5e64` adds otherwise-positive hostile `HostConfig.UTSMode=host` and `HostConfig.CgroupMode=host` fixtures. The current launch plan requests private UTS and cgroup namespaces, but current application-service inspection does not bind those returned values. The owner-local normative rationale and minimum post-RED repair remain in `APPLICATION_SERVICE_NAMESPACE_TRACEABILITY.md`.

Issue #46 / test-bearing `3f7c05b01fb36653e1f13b5cb179881c6e61019f` keeps the immutable request digest fixed while returning a different applied Podman `ImageDigest`. The receipt must fail before port/readiness/lease publication and cleanup must still run. `APPLICATION_SERVICE_IMAGE_DIGEST_TRACEABILITY.md` remains the owner-local authority for the requested-vs-applied image identity distinction.

Issue #47 / test-bearing `115945244d3e7fd2e6a2b04090c16935233a011e` requires a non-empty `ApplicationServiceRequest.command` to replace image ENTRYPOINT as one exact JSON-array argv, preserving argument boundaries and forbidding the same argv from remaining after `IMAGE` as an additional command layer. `APPLICATION_SERVICE_ENTRYPOINT_TRACEABILITY.md` records why launch-plan intent and effective-process proof are separate evidence levels.

The global Gap delta also recorded root issue #24's native-CI trigger history: exact `0f765af1a4eea83029febee3b24c55cd7e7ce4e1` causally exposed stale `push.branches: [main]`, and ordinary root repair `034155c804baffd7b57a97da84a6037d79dd6a96` changed that trigger to protected/default `develop`. That is inherited root evidence, not #19 domain ownership; restoring the #19 global Gap diff must not erase the branch's actual inherited workflow delta or imply that #19 owns root release evidence.

## DDD and evidence levels

`application_service` owns the service request/lease lifecycle and backend-neutral fail-closed contract. `infrastructure::podman` owns Podman-specific inspection and launch-plan translation. `sandbox_execution` owns reusable isolation truth. Requested configuration, backend-applied inspection, and live kernel/process enforcement remain distinct evidence levels; equality at one level must not be promoted to a stronger level without exact runtime proof.

The four #19 lanes are independent. Resource-state, namespace-state, image identity, and process argv must each execute for their intended cause before the corresponding minimum production repair is added. Network attachment/cleanup ownership remains #23/#127 authority, exact application-service lifecycle/destructive identity remains #21/#127 authority, and command-runtime process/isolation controls remain their canonical Core owners.

## Repair decision

Selected repair: keep all focused tests and local TRACEABILITY, add this owner-repair record, then restore only `docs/product-technical-gap-baseline.md` to the exact PR base. Rejected alternatives are copying the newest #121 ledger into this leaf, deleting historical evidence without migration, force-rebasing onto a moving owner, or retaining a second live Gap writer.

After the docs-only head movement, every check must be reacquired on the new exact head. Historical RED/GREEN or queued status does not transfer. Keep the PR Draft until its intended REDs have causal execution and the normal owner/integration/release gates are satisfied.
