# Application-service HTTP-readiness Gap owner repair

Reviewed 2026-09-17 KST against Draft PR #104 exact `4d738ccc52a3acb3d6ea621e306f628074c96122`, exact base `5c6a44bb2b35eb17d0315d72db242f4488c3c426`, and repository-wide Gap owner PR #121.

## Owner-boundary finding

Issue #103 is a focused application-service readiness contract. It owns protocol-aware readiness semantics, the Podman readiness adapter, its focused tests, and the corresponding PRD/TRD/ADR/TRACEABILITY updates. It does not own the repository-wide live product/technical Gap ledger. Review `5229919282` found that the branch still changed `docs/product-technical-gap-baseline.md`; carrying that file forward could replay a 2026-09-09 repository snapshot over #121's newer owner graph.

The repair is migration-first. The protocol evidence and consumer-release rule that had also appeared in the global ledger are retained here and in the existing local PRD/TRD/ADR/TRACEABILITY before the global file is restored byte-for-byte to this PR's exact-base blob `bacb346f2ce4259a4f55bd3bece5e871b06d69db`.

## Retained causal and review evidence

The repaired causal RED `65663052ec30bc178adbe5ff4514f5409d10971f`, native CI `34255729573`, verify `102160927453`, proved that a runtime-owned loopback socket accepting TCP without an HTTP response could incorrectly satisfy `ServiceProtocol::Http`. The first production repair `c05d378cfc736e4257594d69bb06871893ef2d0f` split protocol semantics: `Tcp` remains connect-only; `Http` sends one bounded fixed-path request to the runtime-owned loopback mapping and requires a successful HTTP response class.

Legacy success fixtures that were plain TCP listeners were corrected to `Tcp` in `59cc738f1428d78eaf7a7999e65cc247307b990d`. A focused helper-coverage repair followed without widening readiness semantics. Exact predecessor `d098385045cefd8b337ba2bd0069107a01756b81`, CI `34259463376`, reached hosted GREEN for verify, coverage, branch coverage, and hosted negative rootless/AppArmor; its branch artifact and coverage details remain historical predecessor evidence only.

Code review then identified two narrower protocol-integrity defects: the fixed `Host: 127.0.0.1` omitted the runtime-selected non-default port, and status acceptance treated a prefix beginning with `2` as sufficient. Test-only `32972162bfee95112b2f79a3427ebe4815e24b39`, CI `34267331198`, branch job `102199888859`, executed the causal review RED: the captured request omitted `:<mapped-port>`, while malformed `HTTP/1.1 2x0 ...` and `HTTP/1.1 20x ...` were incorrectly accepted. The same focused binary retained TCP/no-response timeout, HTTP 204 success, and HTTP 503 non-readiness controls.

Minimum production `b481086cbd13a1e94cf8df09d49efb9fa3200e85` derives `Host: 127.0.0.1:<runtime-selected-port>` and validates a bounded HTTP/1.1 three-digit 2xx status prefix. It does not add caller-controlled origin/path, authentication, credential handling, egress policy, model/provider semantics, or new wire fields. RFC 9110/9112 basis and the exact contract distinction remain in `APPLICATION_SERVICE_HTTP_READINESS_TRACEABILITY.md`.

Exact predecessor `4d738ccc52a3acb3d6ea621e306f628074c96122`, native CI `34270054863`, had verify `102209085115`, production coverage `102209084826`, branch coverage `102209084792`, and hosted negative rootless/AppArmor `102209084617` GREEN. Dedicated positive-LSM `102209085130` was still queued and no qualifying approval existed. This hosted GREEN must not transfer after the ownership-only head movement.

## Consumer and release contract retained locally

The PR's PRD/TRD/ADR changes make the ownership boundary normative: consumers must consume an immutable released runtime contract/artifact; mutable PR heads, sibling source copies, direct foreign runtime calls, and cross-service SQL are not integration mechanisms. No GitHub Release is claimed by this branch. Protected integration and immutable version/package/SBOM/provenance/reproducibility/rollback publication remain prerequisites before a consumer version/digest bump.

Caido login/authentication and consumer bootstrap remain consumer-owned. The runtime readiness contract only attests the selected backend-neutral service protocol against the exact runtime-owned loopback mapping.

## DDD and decision

`application_service` owns protocol intent and lease/readiness semantics. `infrastructure::podman` owns the concrete loopback probe translation. `sandbox_execution` remains the reusable isolation owner. Requested protocol intent, successful readiness observation, and positive sandbox confinement are independent evidence dimensions.

Selected repair: preserve all focused source/test/PRD/TRD/ADR/TRACEABILITY changes, add this owner-repair record, and restore only the global Gap file to the exact base. Rejected alternatives are copying #121's latest ledger into this leaf, discarding review/causal history, retaining a second live Gap writer, or force-rebasing merely to remove the file.

After the docs-only movement, all current-head gates must be reacquired. Historical exact-head GREEN remains useful lineage evidence but is not merge/release authority for the moved head.
