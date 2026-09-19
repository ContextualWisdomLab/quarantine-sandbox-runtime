# Podman network identity provenance traceability

## Current finding

The application-service owner now acquires one canonical 64-character lower-case Podman network ID before container creation and uses that ID for the container `--network` selector. That removes the earlier name-only binding path, but the acquisition admission is still incomplete.

`RootlessPodmanAdapter::acquire_network_id` currently deserializes only `id` from the exact-name `podman network inspect` result. Podman exposes the inspected network's `Name`, `ID`, `Internal`, and `DNSEnabled` as distinct properties. A syntactically valid ID therefore becomes backend authority even when the returned object contradicts the generated network name or the deny-by-default state requested during creation.

This is an ownership/provenance boundary, not another random-name-strength problem. Looking up an exact generated name is how the runtime asks Podman for evidence; the returned object still has to prove that the admitted ID describes the intended runtime-created network and retains the P0 configuration that justified admission.

## Current owner-path RED

Exact test-only commit `5d19f2c2c13b6dfc0e3dd49a28c7d3fcf1f88c51` adds `tests/podman_application_service_network_identity_provenance_owner_red.rs` on Draft #127.

The witness keeps the returned `.id` canonical while independently contradicting:

- the inspected network name;
- `internal=true`;
- `dns_enabled=false`.

Every case requires fail-closed rejection before `container create`. The fixture records a consumer-container-create marker if a canonical ID alone is accepted. Production Rust/API/schema/runtime remains unchanged in this commit. This source is RED-only until the exact head executes it for the intended cause; predecessor execution is not transferred.

## Intended minimum repair after causal execution

The smallest owner repair should make network-identity admission one bounded proof rather than several unrelated lookups:

1. deserialize exactly one inspection object with the network `name`, canonical full `id`, `internal`, and `dns_enabled` fields required;
2. require the inspected `name` to equal the exact generated correlation name used for this invocation;
3. require `internal == true` and `dns_enabled == false` before the ID can become private backend authority;
4. admit the canonical full ID only after those provenance/configuration checks pass;
5. keep the generated `qsr-net-*` value as public correlation metadata rather than destructive authority;
6. continue with the already-staged cleanup-authority and exact-container effective-attachment REDs rather than treating identity admission as complete isolation proof.

The future repair must not normalize a contradictory name, silently coerce absent fields to secure defaults, retry until a different object happens to match, or use a larger random name as a substitute for backend evidence.

## Relationship to adjacent owner gaps

This finding is narrower than the existing pre-ID cleanup and post-ID cleanup REDs. If identity admission fails, #127 still has to stop correlation-name network deletion. If identity admission succeeds, the admitted ID still has to survive every partial-launch and termination path and later be matched against the exact acquired container's effective `NetworkSettings.Networks[*].NetworkID` before readiness. Network-level `--force` remains prohibited because Podman documents that force removal also removes containers using that network.

Issue #48 remains the parent P0 authority for acquired network lifecycle identity. Issues #22/#23 own effective attachment, and #41 owns foreign-safe non-force removal. These controls compose; none substitutes for the others.

## Decision record

**Problem.** A canonical network ID without expected-name and P0-state binding is syntactically valid but incomplete ownership evidence.

**Constraints.** Preserve single-writer application-service ownership, current public correlation vocabulary, exact-ID container binding, fail-closed behavior, no mutable sibling dependency, no force cleanup, and no predecessor-GREEN promotion.

**Rejected alternatives.** Larger/randomer names do not establish object identity. Labels alone are mutable metadata and are not sufficient ownership receipts. A second post-create name lookup after admitting the ID widens the TOCTOU surface. Secure Serde defaults turn missing evidence into affirmative evidence and are therefore invalid for P0 admission.

**Selected direction.** Bind expected name and P0 network configuration into the same exact inspection object that yields the immutable network ID, then carry only the admitted ID as private backend authority.

**Effect.** A foreign/rebound or misconfigured object cannot become attachment or cleanup authority merely because it returns a canonical-looking ID.

**Follow-up evidence.** Execute the exact RED; apply the minimum admission repair only after causal execution; then reacquire repository/fmt/full tests/Clippy/public+private rustdoc, complete owned-production line/function/region/branch/edge coverage, effective attachment/cleanup REDs, real rootless network evidence, positive effective LSM, independent review/security, protected integration, and immutable release evidence.

## References

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
