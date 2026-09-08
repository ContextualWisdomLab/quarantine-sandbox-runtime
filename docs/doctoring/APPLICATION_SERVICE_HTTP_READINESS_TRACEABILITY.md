# Application-service HTTP readiness traceability

## Status and owner

Proposed on 2026-09-09. Issue #103 / Draft PR #104 owns the first HTTP-readiness repair inside the existing `application_service` Supporting bounded context and `sandbox_execution` Core lifecycle. It does not create a separate readiness product, move Strix/Caido application authorization into this repository, or assign LLM/provider failure taxonomy to this runtime.

Canonical dependency root for this lane is Draft PR #1 exact `5c6a44bb2b35eb17d0315d72db242f4488c3c426`; protected/default `develop` is `60a85c7633e03b425b67159ec6822c8178cf87ea`.

## Production evidence that motivated the RED

The consumer failure recorded in issue #103 occurred before a Strix model-backed scan when the local Caido endpoint on `127.0.0.1:48080` never became usable. The centralized caller correctly failed closed as `STRIX_SANDBOX_UNAVAILABLE`; adding retries at the consumer would not prove sandbox readiness.

On the canonical root, `RootlessPodmanAdapter::launch_at` verified isolation and the loopback mapping, then called `wait_for_readiness`. Before this lane, that function treated a successful TCP connection as sufficient readiness for both `ServiceProtocol::Tcp` and `ServiceProtocol::Http`.

The buyer-visible gap was therefore narrower than the original standalone #104 prototype: an HTTP application could accept TCP while its HTTP surface was still unavailable, yet the runtime could issue a ready lease.

## Rejected predecessor design

Exact predecessor #104 `cd6316dc7172e97cce2630e698aa947e6df15640` created a second root Cargo package and repository CI directly from protected `develop`. Native CI `34254527569` failed at `cargo test --all-targets` before Clippy or formatting; the branch also declared a missing `src/main.rs` and its test source contained malformed byte-string/JSON literals. That is a repository/test-harness failure, not causal readiness evidence.

The predecessor also exposed caller-controlled arbitrary `origin` and reserved `provider_terminated` / `model_communication_failed` inside the runtime. Those authorities are rejected. The runtime owns runtime-generated loopback service readiness and cleanup. Provider/model communication classification remains with the LLM/orchestration owner.

## Causal RED and minimum behavior repair

Exact `65663052ec30bc178adbe5ff4514f5409d10971f` established the intended RED: a listening TCP socket that deliberately produced no HTTP response was nevertheless accepted as `ServiceProtocol::Http` readiness. The expected contract was `ReadinessTimeout` plus cleanup.

Minimum production candidate `c05d378cfc736e4257594d69bb06871893ef2d0f` changed only readiness semantics:

- `tcp` remains successful loopback transport connection;
- `http` sends a fixed HTTP/1.1 request to `/` on the runtime-derived loopback mapping;
- a final 2xx status class is required before readiness succeeds;
- read/write I/O is bounded by the remaining operator readiness budget;
- no caller-controlled URL, origin, host, path, credential, login, or provider/model classification enters the runtime contract.

Focused regressions prove TCP acceptance alone is not HTTP readiness, HTTP 204 is accepted, and HTTP 503 is not readiness.

## Broad-fixture and coverage RCA

The first exact candidate CI `34257036516` exposed an inherited fixture mismatch: `tests/podman_application_service.rs` used `ServiceProtocol::Http` while its success fixtures were intentionally plain TCP listeners. Commit `59cc738f1428d78eaf7a7999e65cc247307b990d` preserved those process-boundary fixtures as TCP instead of weakening HTTP readiness. Exact CI `34258071256` then made verify GREEN, including the full workspace test suite, Clippy, and rustdoc; hosted negative rootless/AppArmor also passed.

The same exact branch-coverage lane found one direct owned-production admission gap in the new helper: `src/infrastructure/podman.rs` line 889 and four short-circuit branches represented timeout/socket-configuration/write setup failures. Functions were `190/190`, while the helper left total lines `1989/1990`, canonical regions `2664/2665`, and branches `462/466`.

Commit `236d1a67eb90f6d7c10c4714dd2d1d55faee72c6` is the minimum causal repair. It composes read-timeout, write-timeout, request-write, and response-read operations as one ordered `io::Result` chain and returns readiness only when the complete chain succeeds with an HTTP/1.1 2xx prefix. This is not a coverage-threshold exception: every setup/I/O error remains fail closed, but the implementation no longer duplicates one semantic failure outcome across four explicit short-circuit branches. Rust's standard `TcpStream` contract already rejects a zero `Duration` for both read and write timeouts, so removing the separate `timeout.is_zero()` predicate does not create an unbounded-I/O path.

The ordinary coverage lane on `59cc738...` separately reproduced `BackendInvocationFailed { operation: "rootless_probe" }` in `root_coverage_edges`, before its intended `network_inspect` failure. That repeated process-invocation class is owned by Issue #71 / Draft #72. #104 must not hide it with retries, mutexes, environment workarounds, or weakened expectations.

## Review-driven protocol-integrity RED and minimum repair

Code review of hosted-GREEN predecessor `d098385045cefd8b337ba2bd0069107a01756b81` found two acceptance defects in the probe itself. The request used a fixed `Host: 127.0.0.1` even though the actual runtime authority is the dynamically selected `127.0.0.1:<mapped-port>`. The response check read only `HTTP/1.1 2`, so malformed status codes whose first character was `2` could be promoted to readiness.

Test-only exact `32972162bfee95112b2f79a3427ebe4815e24b39` made both findings causal. Native CI `34267331198`, branch-coverage job `102199888859`, reached `tests/application_service_http_readiness_red.rs` and failed exactly three focused controls: the captured request omitted the selected port from `Host`, and malformed `HTTP/1.1 2x0 ...` / `HTTP/1.1 20x ...` responses both produced `Ok(ApplicationServiceLease)` instead of `ReadinessTimeout`. Existing TCP-only timeout, valid HTTP 204, and HTTP 503 controls remained GREEN in the same test binary.

Minimum descendant `b481086cbd13a1e94cf8df09d49efb9fa3200e85` changes only `src/infrastructure/podman.rs`:

- the fixed request constant is replaced by a request generated from the already-validated runtime-selected loopback port, producing `Host: 127.0.0.1:<mapped-port>`;
- the probe reads a bounded 13-byte HTTP/1.1 status prefix and requires `HTTP/1.1 `, a literal `2`, two ASCII decimal digits, and the required following space;
- existing bounded read/write timeout chaining, fixed `/` path, TCP connect-only readiness, cleanup, and consumer-neutral ownership remain unchanged.

RFC 9110 section 7.2 defines `Host = uri-host [ ":" port ]` as target authority information. RFC 9112 requires an HTTP/1.1 `Host` field consistent with the target URI authority and defines the status line as HTTP-version, space, a three-digit status code, space, and optional reason phrase. The repair therefore tightens the probe to the protocol grammar without adding application authentication semantics.

## DDD and security decision

`application_service` owns the consumer-neutral meaning of an HTTP-ready leased service. `sandbox_execution` owns bounded readiness/lifecycle and cleanup truth. `infrastructure` performs concrete socket/HTTP I/O against only the runtime-generated loopback endpoint. Podman, Docker/Colima-compatible OCI execution, gVisor/containerd, or Kubernetes adapters must not alter the readiness domain contract merely because the backend changes.

An arbitrary origin is not accepted. The probe target is derived from the already-validated runtime-owned loopback endpoint. Application-specific authentication semantics remain typed Supporting-context configuration or consumer ACL data rather than Core UL.

RFC 9110 defines 2xx as the successful response class. The P0 probe uses only that status class rather than embedding application-specific response bodies or login semantics. The current wire shape therefore remains unchanged.

## Evidence and release gates

This lane is not release authority until its final exact head reacquires repository validation, fmt/test/Clippy/rustdoc, 100% owned-production statement/function/source-region/branch coverage, hosted negative and real positive effective-LSM evidence where applicable, review/security gates, protected integration, SBOM/provenance/reproducibility/rollback, and immutable publication before any consumer version bump.

## References

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics (RFC 9110).* RFC Editor. https://www.rfc-editor.org/rfc/rfc9110.html

Thomson, M., & Nottingham, M. (2022). *HTTP/1.1 (RFC 9112).* RFC Editor. https://www.rfc-editor.org/rfc/rfc9112.html

Rust Project Developers. (2026). *TcpStream in std::net*. Rust standard library documentation. https://doc.rust-lang.org/std/net/struct.TcpStream.html

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190