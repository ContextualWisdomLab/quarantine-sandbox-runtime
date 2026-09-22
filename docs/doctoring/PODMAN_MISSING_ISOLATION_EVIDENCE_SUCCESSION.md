# Podman missing isolation evidence succession

## Authority

This succession lane is based on canonical command/runtime owner #112 exact `19c450249e1779cd1d22c4837c8154fed91e1faf`. It adopts the valid security contract proven by historical focused owner #89 exact `8afa77c61af38121cbfcdc1051e6c45376e13a1b` / CI `35546707858` without copying that stale whole-head ancestry.

#89 current exact executed. Exact-head verify passed locked workspace/all-target tests, Clippy `-D warnings`, and public/private rustdoc; hosted negative rootless/AppArmor acceptance passed. Nightly branch coverage executed its focused missing-evidence cases before whole-head admission failed. Historical whole-head coverage remains below repository-wide 100% and positive SELinux never received a runner, so #89 itself is not merge/release-ready.

The owner-adapted RED executed at #143 exact `0bf65d831f2130dbf414fef96cfbfa2828aabfdd` / CI `35685623657`. Production repair exact `f65d0982c756b67c85b3e60f3649e3ca201d50ca` is ordinary owner-authored ancestry, not a workflow-authored source mutation.

## Problem

At #112 exact `19c450249...`, `ContainerInspection.EffectiveCaps` and `ContainerInspection.BoundingCaps` used both `#[serde(default)]` and `deserialize_with = "null_as_default"`, while `NetworkInspection.dns_enabled` defaulted on absence. Missing backend evidence could therefore become the secure values `[]`, `[]`, and `false` before the infrastructure ACL evaluated the P0 controls.

The owner also documents a local observation that Podman 6.1.0 emitted explicit JSON `null` for dropped capability arrays. That observation is not yet a sufficient security contract. Podman v6.1.0 documents `EffectiveCaps` and `BoundingCaps` as arrays, and the upstream inspect data model represents them as `[]string`. A nil Go slice can serialize as JSON `null`, but serialization alone does not prove that Podman positively observed an empty configured capability set rather than leaving the slice unpopulated. Until exact-version real-runtime evidence establishes the narrower meaning, `null` is unavailable/ambiguous isolation evidence and must fail closed.

## RED and minimum GREEN

`tests/podman_missing_isolation_evidence_succession_red.rs` serializes its fake-process fixtures and requires:

- missing `EffectiveCaps` -> `MalformedIsolationInspection { operation: "container_inspect" }`;
- missing `BoundingCaps` -> the same container-inspection failure;
- missing `dns_enabled` -> `MalformedIsolationInspection { operation: "network_inspect" }`;
- explicit empty capability arrays remain admissible and reach a deterministic malformed-port boundary;
- explicit JSON `null` for `EffectiveCaps` independently fails as malformed container inspection unless a separately proven, exact-version compatibility rule is introduced;
- explicit JSON `null` for `BoundingCaps` independently fails under the same rule.

The two explicit-null controls are intentionally independent. A combined fixture with both fields set to `null` can false-GREEN if parsing rejects only the first field encountered while the other field remains incorrectly normalized to secure empty evidence. Each configured capability source therefore has its own hostile representation witness.

The deterministic positive control deliberately returns a malformed port mapping and expects `InvalidPortMapping`. It does not reserve and release an ephemeral loopback port before readiness, so this parser/ACL witness has no ambient port-race dependency.

The minimum owner-safe GREEN is to require concrete arrays for `EffectiveCaps` and `BoundingCaps` and a concrete boolean for `dns_enabled`: remove secure missing-key defaults and do not normalize JSON `null` to an empty capability set. If a supported Podman version truly requires a `null` compatibility exception, add it only after real rootless evidence records the exact Podman version, exact P0 create configuration, raw inspect JSON, and independent live process capability sets proving effective/bounding/permitted/inheritable/ambient capabilities are empty. The exception must be version-scoped and must not convert general absence or malformed evidence into a secure value.

This RED deliberately does not assert network cleanup command shape. Network identity, recovery, and exact-ID/non-force lifecycle remain #127/#141/#142 authority.

## Executed successor RED — 2026-09-22

Exact `0bf65d831f2130dbf414fef96cfbfa2828aabfdd` / native CI `35685623657` received GitHub-hosted runners and executed all six focused controls under branch instrumentation. The explicit-empty capability control reached the deterministic downstream `InvalidPortMapping` boundary. Each unavailable-evidence case instead also reached `InvalidPortMapping`, where the witness required a malformed inspection: missing `EffectiveCaps`, missing `BoundingCaps`, missing `dns_enabled`, `EffectiveCaps: null`, and `BoundingCaps: null`. The shared downstream outcome proved the production parser was still converting unavailable configured isolation evidence into secure values before the ACL evaluated it.

Verify on the same exact passed exact checkout, dependency lock, repository policy, and CI-contract checks before stopping at rustfmt in the new witness. Formatter-only descendant `7d2b1f7c10a7aa70d9b7e70158c5796c277574fc` changed no semantics. The later owner-path cleanup removed the rejected self-mutating source-fix workflow; it did not change production Rust. Docs-only exact `fadd477029908dfa636e63730fef3ebb7125973b` then recorded the executed RED before production moved.

## Owner-authored production repair — 2026-09-22

Exact `f65d0982c756b67c85b3e60f3649e3ca201d50ca` applies the minimum causal production repair directly on ordinary owner ancestry. `EffectiveCaps` and `BoundingCaps` are required concrete `Vec<String>` fields, `dns_enabled` is a required concrete boolean, the generic `null_as_default` helper and its stale compatibility rustdoc are removed, and the now-unused `Deserializer` import is removed. Explicit empty arrays remain representable observations. Missing keys and explicit JSON `null` no longer normalize to secure empty capability evidence through this DTO.

Docs-only exact `3c2c4145a9e0625c663141b5808daa34f0a305eb` made this TRACEABILITY code-current with that production repair and triggered native CI `35722118341`.

## Executed repair validation and fixture migration — 2026-09-22

CI `35722118341` received GitHub-hosted runners. The six focused parser controls are GREEN on the repaired production semantics: the explicit-empty capability control still reaches deterministic `InvalidPortMapping`, while missing `EffectiveCaps`, missing `BoundingCaps`, missing `dns_enabled`, explicit `EffectiveCaps:null`, and explicit `BoundingCaps:null` now fail at the intended malformed inspection boundary. Hosted rootless/AppArmor negative acceptance is also GREEN.

The broad verify/coverage failures on that exact are a separate fixture-migration defect rather than evidence that the production parser repair is wrong. Several tests whose purpose is LSM, process-top, cleanup, output, wait, mount, or lifecycle behavior still supplied `EffectiveCaps:null` / `BoundingCaps:null` in fixtures intended to represent a valid secure container inspection. The new fail-closed parser correctly rejected those fixtures before their tests could reach the boundary each test was meant to exercise.

The active owner therefore migrated only valid-path test data to explicit configured empty capability arrays. Ordinary commits repair the shared command fake, backend evidence helper, live-attestation, source mount-set, output-encoding, post-create ownership, wait-output-budget, runtime-evidence-cardinality, source-mount-type, and general command-execution fixtures. In the runtime-evidence-cardinality suite, the nonempty-capability mutations were updated to mutate from explicit `[]`, preserving their independent negative evidence. Production Rust and the five hostile missing/null witnesses remain unchanged.

Current exact `61eefd2029c390bed6d3a434dd0ad1aa662763af` contains that complete fixture migration identified by CI `35722118341`; native CI `35739891481` is the first exact-head run capable of validating the repaired parser together with the corrected valid-path fixtures. No predecessor result transfers to this moved exact. Positive self-hosted SELinux remains an independent required runtime lane.

The next action is validation, not semantic widening. Reacquire repository policy, rustfmt, full locked workspace/all-target tests, Clippy `-D warnings`, public/private rustdoc, 100% owned-production statement/function/region/branch/edge coverage, hosted negative runtime acceptance, and applicable positive selected-LSM evidence on one unchanged successor. Any future explicit-null compatibility exception remains subject to the exact-version evidence rule above.

## DDD and evidence boundary

`infrastructure::podman` is the Anti-Corruption Layer translating backend-specific inspection documents into application-service isolation evidence. Presence and representation admission belong at this boundary. `application_service` and `sandbox_execution` continue to consume backend-neutral failures and verified states; no Podman DTO leaks inward.

Fake-Podman fixtures prove parsing/ACL behavior only. Release authority still requires one unchanged integrated candidate with repository validation, rustfmt, locked tests, Clippy, public/private rustdoc, 100% owned-production statement/function/region/branch and edge coverage, qualifying review/security evidence, real rootless runtime acceptance, positive selected-LSM evidence, protected release integration, SBOM/provenance/reproducibility/rollback, and immutable publication.

## References

Podman Authors. (n.d.). *podman-container-inspect — Display a container's configuration* (Podman v6.1.0). Podman documentation. https://docs.podman.io/en/v6.1.0/markdown/podman-container-inspect.1.html

Podman Authors. (n.d.). *Podman REST API reference* (v6.1.0). Podman documentation. https://docs.podman.io/en/v6.1.0/_static/api.html

Podman Authors. (n.d.). *container_inspect.go* (v6.1.0). Podman source. https://github.com/containers/podman/blob/v6.1.0/libpod/define/container_inspect.go

Podman Authors. (n.d.). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Serde Project. (n.d.). *Default value for a field*. Serde documentation. https://serde.rs/attr-default.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
