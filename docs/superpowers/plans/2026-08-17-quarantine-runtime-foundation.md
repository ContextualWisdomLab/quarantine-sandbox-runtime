# Quarantine Runtime Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a source-agnostic, credential-free Rust foundation that converts bounded artifact bytes into deterministic evidence without executing content or deciding maliciousness.

**Architecture:** A contract module validates hostile metadata, an ingestion module establishes immutable identity and classifies file families, and a runtime module invokes ordered static analyzers and assembles an attributable evidence bundle. Consumer systems such as Wardnet own verdict and response policy.

**Tech Stack:** Rust 1.97.1, edition 2024, serde, serde_json, sha2, thiserror, JSON Schema Draft 2020-12, cargo-llvm-cov, GitHub Actions.

## Global Constraints

- The runtime must not expose a malicious/benign verdict.
- The foundation must not execute artifacts, make network requests, or receive credentials.
- `unsafe` is forbidden.
- Public Rust API documentation is mandatory.
- Same request and bytes must produce deterministic identifiers and evidence order.
- GitHub Actions must be pinned by full commit SHA.
- Production statement, function, region, and branch coverage target 100%.

---

### Task 1: Commit the failing public-contract tests

**Files:**
- Create: `tests/contracts.rs`
- Create: `tests/ingestion.rs`
- Create: `tests/runtime.rs`
- Create: `src/lib.rs`
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`

**Interfaces:**
- Consumes: none.
- Produces: the required public API names and behavior encoded by integration tests.

- [ ] **Step 1: Add tests for wire codes, validation, ingestion, analyzers, evidence ordering, and fail-closed dynamic profiles.**
- [ ] **Step 2: Run `cargo test --workspace --all-targets`.**
- [ ] **Step 3: Confirm failure is caused by unresolved public API imports rather than malformed test syntax.**
- [ ] **Step 4: Commit with `test: define quarantine runtime contracts`.**

### Task 2: Implement versioned domain contracts

**Files:**
- Create: `src/contracts.rs`
- Modify: `src/lib.rs`
- Test: `tests/contracts.rs`

**Interfaces:**
- Consumes: contract expectations from Task 1.
- Produces: `AnalysisRequest::validate`, `ArtifactDescriptor::validate`, `EvidenceBundle::validate`, stable enum wire codes, and `ContractError`.

- [ ] **Step 1: Run `cargo test --test contracts` and retain the unresolved-import failure evidence.**
- [ ] **Step 2: Add serde-backed enums and documented structures with bounded validation.**
- [ ] **Step 3: Run `cargo test --test contracts` and require all contract tests to pass.**
- [ ] **Step 4: Commit with `feat: add versioned analysis contracts`.**

### Task 3: Implement bounded artifact ingestion

**Files:**
- Create: `src/ingestion.rs`
- Modify: `src/lib.rs`
- Test: `tests/ingestion.rs`

**Interfaces:**
- Consumes: `ArtifactDescriptor` and `ArtifactKind` from Task 2.
- Produces: `IngestionPolicy`, `IngestionError`, `IngestedArtifact`, and `ingest_bytes`.

- [ ] **Step 1: Run `cargo test --test ingestion` and retain the unresolved-import failure evidence.**
- [ ] **Step 2: Add hard bounds, SHA-256 identity, original-byte preservation, and ordered magic-byte classification.**
- [ ] **Step 3: Run `cargo test --test ingestion` and require all ingestion tests to pass.**
- [ ] **Step 4: Commit with `feat: add bounded immutable ingestion`.**

### Task 4: Implement evidence-producing runtime orchestration

**Files:**
- Create: `src/runtime.rs`
- Modify: `src/lib.rs`
- Test: `tests/runtime.rs`

**Interfaces:**
- Consumes: validated requests and ingested artifacts.
- Produces: `StaticAnalyzer`, `AnalyzerFinding`, `AnalyzerFailure`, `FormatAnalyzer`, `AnalysisEngine`, `AnalysisError`, and `to_pretty_json`.

- [ ] **Step 1: Run `cargo test --test runtime` and retain the unresolved-import failure evidence.**
- [ ] **Step 2: Implement deterministic job IDs, evidence IDs, analyzer ordering, failure evidence, boundary manifest, and dispositions.**
- [ ] **Step 3: Run `cargo test --test runtime` and require all runtime tests to pass.**
- [ ] **Step 4: Run the full workspace test suite.**
- [ ] **Step 5: Commit with `feat: assemble deterministic evidence bundles`.**

### Task 5: Enforce quality, schemas, and security documentation

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `scripts/validate_repository.py`
- Create: `schemas/analysis-request.schema.json`
- Create: `schemas/evidence-bundle.schema.json`
- Create: `docs/PRD.md`
- Create: `docs/TRD.md`
- Create: `docs/ARCHITECTURE.md`
- Create: `docs/SECURITY.md`
- Create: `docs/THREAT_MODEL.md`
- Create: `docs/TEST_STRATEGY.md`
- Create: `docs/OPERABILITY.md`
- Create: `docs/adr/*.md`
- Create: `docs/doctoring/*.md`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: all public wire contracts.
- Produces: reviewed specifications, strict schemas, policy checks, exact-head quality gates, and release traceability.

- [ ] **Step 1: Run `python3 scripts/validate_repository.py`.**
- [ ] **Step 2: Generate and commit `Cargo.lock`.**
- [ ] **Step 3: Run `cargo fmt --check`.**
- [ ] **Step 4: Run `cargo clippy --locked --workspace --all-targets -- -D warnings`.**
- [ ] **Step 5: Run `RUSTDOCFLAGS=-D warnings cargo doc --locked --workspace --no-deps`.**
- [ ] **Step 6: Run complete line, function, region, and branch coverage gates.**
- [ ] **Step 7: Commit with `docs: establish quarantine runtime assurance baseline`.**
