# Application-service HTTP readiness traceability

## Status and owner

Proposed on 2026-09-09. Issue #103 / Draft PR #104 owns the first HTTP-readiness repair inside the existing `application_service` Supporting bounded context and `sandbox_execution` Core lifecycle. It does not create a separate readiness product, move Strix/Caido application authorization into this repository, or assign LLM/provider failure taxonomy to this runtime.

Canonical dependency root for this RED is Draft PR #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`; protected/default `develop` is `60a85c7633e03b425b67159ec6822c8178cf87ea`.

## Production evidence that motivates the RED

The consumer failure recorded in issue #103 occurred before a Strix model-backed scan when the local Caido endpoint on `127.0.0.1:48080` never became usable. The centralized caller correctly failed closed as `STRIX_SANDBOX_UNAVAILABLE`; adding retries at the consumer would not prove sandbox readiness.

On the canonical root, `RootlessPodmanAdapter::launch_at` verifies isolation and the loopback mapping, then calls `wait_for_readiness`. That function treats a successful TCP connection as sufficient readiness for both `ServiceProtocol::Tcp` and `ServiceProtocol::Http`. The current TRD explicitly documents this P0 behavior and says a future typed HTTP health contract may refine it.

The buyer-visible gap is therefore narrower than the original standalone #104 prototype: an HTTP application can accept TCP while its HTTP/login surface is still unavailable, yet the runtime can issue a ready lease.

## Rejected predecessor design

Exact predecessor #104 `cd6316dc7172e97cce2630e698aa947e6df15640` created a second root Cargo package and repository CI directly from protected `develop`. Native CI `34254527569` failed at `cargo test --all-targets` before Clippy or formatting; the branch also declared a missing `src/main.rs` and its test source contained malformed byte-string/JSON literals. That is a repository/test-harness failure, not causal readiness evidence.

The predecessor also exposed caller-controlled arbitrary `origin` and reserved `provider_terminated` / `model_communication_failed` inside the runtime. Those authorities are rejected. The runtime owns runtime-generated loopback service readiness and cleanup. Provider/model communication classification remains with the LLM/orchestration owner.

## Current RED contract

`tests/application_service_http_readiness_red.rs` uses the existing application-service request, isolation policy, and Podman ACL. It supplies otherwise-positive backend/isolation evidence and a runtime-mapped loopback port with a listening TCP socket that deliberately produces no HTTP response.

Required behavior:

- `ServiceProtocol::Http` must not become ready from TCP acceptance alone;
- readiness failure remains bounded by the operator readiness budget;
- a failed readiness attempt must stop/remove the runtime-owned container and remove its network;
- `ServiceProtocol::Tcp` semantics remain a separate capability and are not implicitly upgraded to HTTP.

The RED intentionally does not yet prescribe Caido's `loginAsGuest`, an arbitrary caller URL, or provider/model failures. After this exact test executes for the intended cause, the minimum GREEN must introduce only the backend-neutral HTTP handshake capability needed to distinguish protocol readiness from TCP reachability. Any request/wire-shape change requires versioned schema compatibility and PRD/TRD/ADR-0006 synchronization before merge.

## DDD and security decision

`application_service` owns the consumer-neutral meaning of an HTTP-ready leased service. `sandbox_execution` owns bounded readiness/lifecycle and cleanup truth. `infrastructure` performs concrete socket/HTTP I/O against only the runtime-generated loopback endpoint. Podman, Docker/Colima-compatible OCI execution, gVisor/containerd, or Kubernetes adapters must not alter the readiness domain contract merely because the backend changes.

An arbitrary origin is not accepted. The probe target must be derived from the already-validated runtime-owned loopback endpoint. Application-specific authentication semantics remain typed Supporting-context configuration or consumer ACL data rather than Core UL.

## Evidence and release gates

This RED is not release authority. A future GREEN still requires exact-head repository validation, fmt/test/Clippy/rustdoc, 100% owned-production statement/function/source-region/branch coverage, hosted negative and real positive effective-LSM evidence where applicable, review/security gates, protected integration, SBOM/provenance/reproducibility/rollback, and immutable publication before any consumer version bump.

## References

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP Semantics (RFC 9110).* RFC Editor. https://www.rfc-editor.org/rfc/rfc9110.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application Container Security Guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
