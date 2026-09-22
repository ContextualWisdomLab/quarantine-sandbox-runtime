# Podman missing isolation evidence succession

## Authority

This dependent RED is based on canonical command/runtime owner #112 exact `19c450249e1779cd1d22c4837c8154fed91e1faf`. It adopts the valid security contract proven by historical focused owner #89 exact `8afa77c61af38121cbfcdc1051e6c45376e13a1b` / CI `35546707858` without copying that stale whole-head ancestry.

#89 current exact executed. Exact-head verify passed locked workspace/all-target tests, Clippy `-D warnings`, and public/private rustdoc; hosted negative rootless/AppArmor acceptance passed. Nightly branch coverage executed all four focused missing-evidence cases successfully. Historical whole-head coverage remains below repository-wide 100% and positive SELinux never received a runner, so #89 itself is not merge/release-ready.

## Problem

At #112 exact `19c450249...`, `ContainerInspection.EffectiveCaps` and `ContainerInspection.BoundingCaps` still use `#[serde(default)]`, while `NetworkInspection.dns_enabled` also defaults on absence. Missing backend evidence can therefore become the secure values `[]`, `[]`, and `false` before the infrastructure ACL evaluates the P0 controls.

The current owner also contains a newer compatibility rule absent from #89: Podman 6.1.0 was observed to emit an explicit JSON `null` for dropped capability arrays. `null_as_default` intentionally maps that explicit value to an empty set. Succession must preserve this observed representation while restoring presence admission. A missing key and an explicitly supplied `null` are not the same backend observation.

## RED and minimum GREEN

`tests/podman_missing_isolation_evidence_succession_red.rs` serializes its fake-process fixtures and requires:

- missing `EffectiveCaps` -> `MalformedIsolationInspection { operation: "container_inspect" }`;
- missing `BoundingCaps` -> the same container-inspection failure;
- missing `dns_enabled` -> `MalformedIsolationInspection { operation: "network_inspect" }`;
- explicit empty arrays and explicit JSON `null` capability arrays remain admissible parsing evidence and continue to the independent readiness gate.

The minimum owner-adapted GREEN is therefore narrower than source-copying #89: remove only the `default` behavior from the two capability fields while retaining `deserialize_with = "null_as_default"`, and make `dns_enabled` a required boolean. Do not infer missing values from process-top evidence or unrelated configuration.

This RED deliberately does not assert network cleanup command shape. Network identity, recovery, and exact-ID/non-force lifecycle remain #127/#141/#142 authority.

## DDD and evidence boundary

`infrastructure::podman` is the Anti-Corruption Layer translating backend-specific inspection documents into application-service isolation evidence. Presence admission belongs at this boundary. `application_service` and `sandbox_execution` continue to consume backend-neutral failures and verified states; no Podman DTO leaks inward.

Fake-Podman fixtures prove parsing/ACL behavior only. Release authority still requires one unchanged integrated candidate with repository validation, rustfmt, locked tests, Clippy, public/private rustdoc, 100% owned-production statement/function/region/branch and edge coverage, qualifying review/security evidence, real rootless runtime acceptance, positive selected-LSM evidence, protected release integration, SBOM/provenance/reproducibility/rollback, and immutable publication.

## References

Podman Authors. (n.d.). *podman-container-inspect — Display a container's configuration*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-container-inspect.1.html

Podman Authors. (n.d.). *podman-network-inspect — Display the network configuration for one or more networks*. Podman documentation. https://docs.podman.io/en/latest/markdown/podman-network-inspect.1.html

Serde Project. (n.d.). *Default value for a field*. Serde documentation. https://serde.rs/attr-default.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
