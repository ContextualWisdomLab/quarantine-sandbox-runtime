# Podman network identity provenance traceability

## Current finding

The application-service owner now acquires one canonical 64-character lower-case Podman network ID before container creation and uses that ID for the container `--network` selector. That removes the earlier name-only binding path, but the acquisition admission remains incomplete in two distinct ways.

First, the inspection object must be internally authoritative: `RootlessPodmanAdapter::acquire_network_id` historically deserialized only `id` from the exact-name `podman network inspect` result. Podman exposes the inspected network's `Name`, `ID`, `Internal`, and `DNSEnabled` as distinct properties. A syntactically valid ID must not become backend authority when the returned object contradicts the generated network name or the deny-by-default state requested during creation.

Second, even a self-consistent first inspection does not by itself prove that the inspected object is the object created by this invocation. The Podman CLI `network create` contract reports the newly created network **name** on success. The current owner then performs a separate name-based inspection to learn the immutable ID. A same-name replacement between create and that first inspection can therefore present a different canonical ID while preserving the expected name, `internal=true`, and DNS-disabled state. The present code would promote that replacement ID to private attachment authority and bind container creation to it.

This is an ownership/provenance boundary, not another random-name-strength problem. High-entropy names reduce accidental collision and blind guessing; they do not convert a later name lookup into creation-bound immutable evidence.

## Current owner-path REDs

Test-only commit `5d19f2c2c13b6dfc0e3dd49a28c7d3fcf1f88c51` added `tests/podman_application_service_network_identity_provenance_owner_red.rs` on Draft #127. It keeps the returned `.id` canonical while independently contradicting the inspected network name, `internal=true`, and `dns_enabled=false`. Every case requires fail-closed rejection before `container create`.

Review `5266446082` identifies the separate create→first-inspect TOCTOU gap. Test-only commit `d4f05af6cdf5a31b36783bff53839530600b5e48` adds `tests/podman_application_service_network_create_inspect_toctou_red.rs`. Its fake backend makes `network create` succeed and return the generated correlation name, then makes the first exact-name inspection return a different canonical network ID with the same expected name, `internal=true`, and DNS disabled. The witness requires failure before any container creation can consume that replacement ID.

Production Rust/API/schema/runtime remains unchanged by these RED commits. The new TOCTOU witness is not GREEN authority until its exact head executes and reaches the intended assertion; predecessor execution does not transfer.

## Causal repair boundary

Do not choose a production mechanism before the create→first-inspect RED executes. In particular, merely adding more predicates to the later name-based inspection is not sufficient: the hostile replacement can satisfy the same name and P0-state predicates.

After causal execution, the minimum acceptable GREEN must provide creation-bound evidence that cannot be reconstructed from the public correlation name alone. Candidate mechanisms must be evaluated against the actual Podman interface and include one of the following properties:

1. the create operation itself returns an immutable network identity that the runtime can admit directly; or
2. a backend transaction/API provides an equivalent atomic create-and-return-identity contract; or
3. another independently justified receipt binds the immutable ID to the exact creation operation without promoting name/prefix/label/age/dangling metadata by itself to destructive authority.

If the CLI cannot provide such evidence, that is an adapter limitation to document and repair at the infrastructure boundary rather than a reason to weaken the domain invariant. A runtime-generated label may be useful corroboration, but mutable/reproducible metadata alone is not an immutable creation receipt.

Once creation-bound identity exists, admission must still require exactly one object, canonical full ID, expected P0 state, exact-container effective attachment by that ID, private lifecycle retention, and non-force cleanup. The generated `qsr-net-*` name remains consumer-visible correlation metadata.

## Relationship to adjacent owner gaps

This finding is narrower than #141 recovery but precedes later network authority. If creation-bound identity cannot be admitted, no generated-name deletion is authorized and #141 owns durable orphan reconciliation. If identity admission succeeds, the admitted ID still has to survive every partial-launch and termination path and later be matched against the exact acquired container's effective `NetworkSettings.Networks[*].NetworkID` before readiness. Network-level `--force` remains prohibited because it can affect containers attached to that network.

Issue #48 remains the parent P0 authority for acquired network lifecycle identity. Issues #22/#23 own effective attachment, #41 owns foreign-safe non-force removal, and #141 owns no-admitted-ID recovery. These controls compose; none substitutes for creation-bound identity.

## Decision record

**Problem.** A canonical network ID learned through a later name lookup is syntactically valid but is not necessarily the immutable identity of the object created by this invocation.

**Constraints.** Preserve single-writer application-service ownership, current public correlation vocabulary, fail-closed behavior, no mutable sibling dependency, no force cleanup, no predecessor-GREEN promotion, and the rule that correlation/discovery metadata cannot independently become destructive capability.

**Rejected alternatives.** Larger/randomer names reduce guessing but do not establish object identity. Expected-name plus P0-state checks reject contradictory inspections but do not distinguish a conforming same-name replacement. Labels alone are mutable metadata and are not sufficient ownership receipts. Retrying name inspection widens the race and can eventually select a different object. Secure Serde defaults turn missing evidence into affirmative evidence and remain invalid for P0 admission.

**Selected direction.** Establish causal RED first. The eventual adapter repair must bind immutable network identity to the creation operation itself, then use later inspection only as corroborating state/effective-configuration evidence rather than as the sole source of ownership.

**Effect.** A same-name replacement cannot become attachment or cleanup authority merely by winning the interval between network creation and the first lookup.

**Follow-up evidence.** Execute the exact RED; classify the first failure; select the smallest creation-bound mechanism supported by the backend; reacquire repository/fmt/full tests/Clippy/public+private rustdoc, complete owned-production line/function/region/branch/edge coverage, effective attachment/cleanup REDs, real rootless network evidence, positive effective LSM, independent review/security, protected integration, and immutable release evidence.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/stable/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-create — Create a new container*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-create.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-rm.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
