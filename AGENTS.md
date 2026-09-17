# Agent Development Rules

## Product authority

This repository owns reusable sandbox execution, isolation policy enforcement, resource bounds, lease/readiness/cleanup attestation, and artifact-analysis evidence.

It does not own Wardnet verdict/incident/quarantine policy or Chat/Agent conversation/task/tool/application authorization/secrets. Keep consumer authority behind versioned Anti-Corruption Layers.

## DDD

- `sandbox_execution` is the Core bounded context.
- `artifact_analysis` and `application_service` are Supporting bounded contexts.
- Podman/gVisor/containerd/Kubernetes/VM implementations are infrastructure adapters.
- Do not put unrelated domain behavior in root `utils`, `helpers`, `services`, `common`, `core`, `models`, or similar generic buckets.
- Do not reintroduce root `contracts.rs`, `ingestion.rs`, or `runtime.rs` for artifact-analysis behavior; implementation belongs under `src/artifact_analysis/`.
- Domain contracts must not depend on consumer repository types or container backend SDK/CLI DTOs.
- No direct foreign application-table SQL or sibling source vendoring.
- If a new stable responsibility does not fit a bounded context, update Context Map/ADR before implementation.

## Security invariants

- Standard application-service images are immutable SHA-256 digest references and launch with no implicit pull.
- Rootless execution is required for the Podman P0 profile.
- Read-only rootfs, bounded tmpfs, all capabilities dropped, no-new-privileges, isolated namespaces, non-root UID/GID, resource limits, internal network, loopback-only publication, readiness, and cleanup are fail-closed contracts.
- No P0 secrets, arbitrary environment variables, host devices, runtime sockets, privileged mode, host namespaces, broad host mounts, wildcard/public service bind, or arbitrary Internet egress.
- Artifact bytes are never executed in static paths.
- Static evidence is never labeled observed runtime behavior.
- Do not suppress security/deprecation/toolchain failures to obtain a green check.

## TDD and validation

Behavior changes require RED → smallest causal GREEN → refactor → focused/full verification.

Before merge:

- `cargo fmt --check`
- `cargo test --locked --workspace --all-targets`
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps`
- `python3 scripts/validate_repository.py`
- exact 100% owned production statement/function/region/branch coverage where tooling exposes it;
- all required SAST/security/dependency/SBOM/provenance/review gates;
- real rootless-container E2E for any release claim about actual container isolation.

Fake-Podman/process tests do not substitute for real Podman isolation evidence.

## Documentation

Keep README, PRD, TRD, ARCHITECTURE, ADRs, SECURITY, THREAT_MODEL, TEST_STRATEGY, OPERABILITY, `docs/product-technical-gap-baseline.md`, doctoring/traceability, schemas, and CHANGELOG code-current. Distinguish protected-branch truth from active PR/planned capabilities.

## Database

There is currently no durable database. If persistence is introduced, add an ADR first; use 3NF, descriptive two-or-more-word `snake_case` database object names, tenant/time/concurrency/idempotency invariants, migrations, backup/restore and retention evidence.

## Consumer integration

Use immutable published artifacts/contracts. Do not modify Wardnet or contextual-orchestrator source from this repository's dedicated writer when those repositories have their own writer/owner path. Route consumer work through their existing issue/PR/task path and continue local runtime work.
