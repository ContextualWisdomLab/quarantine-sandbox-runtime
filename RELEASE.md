# Release Runbook

Quarantine Sandbox Runtime releases are security-boundary deliveries. A version is not releasable because a package can be built; the exact protected source commit, hostile-runtime acceptance, package bytes, SBOM, checksums, provenance, review, and release metadata must identify the same candidate.

## Authority and flow

The repository currently uses Git Flow semantics: `develop` is the live integration/default branch and `main` is the stable release branch. Repository metadata and organization rules are re-read before every release. A release tag is valid only when it points to the exact current protected `main` commit. The release workflow checks this rather than trusting branch names remembered from prior runs.

No release is created from a pull-request head, feature branch, predecessor check, queued job, skipped-required job, or unprotected stable branch. Administrator bypass is not release evidence.

## Candidate preparation

Before creating a tag:

1. Integrate the complete dependency stack through the then-live protected repository flow with all required reviews, resolved threads, CI, Security Scan, SAST, and organization workflows terminal-success on unchanged exact heads.
2. Require the final protected candidate to retain 100% owned production statement and branch coverage, warning-free rustdoc, repository-policy validation, and the real hostile-workload isolation acceptance.
3. Promote `CHANGELOG.md` content from `Unreleased` to `## [X.Y.Z] - YYYY-MM-DD` and make `Cargo.toml` package version exactly `X.Y.Z` in a reviewed candidate change. Do not rewrite a historical release section.
4. Confirm application-service and artifact-analysis consumer contracts are versioned and backward-compatibility expectations are documented. Consumers must never pin a transient PR head for production.
5. Confirm Wardnet and contextual-orchestrator owner paths are still blocked from consuming unreleased runtime source.
6. Confirm the dedicated hostile-workload runner described by `ContextualWisdomLab/.github#1590` is available. The first reviewed runner profile is rootless Podman on a disposable SELinux-capable Linux host. A different stronger profile requires its own reviewed security decision.

## Tag and release workflow

Create an annotated or lightweight `vX.Y.Z` tag only after the candidate preparation above. `.github/workflows/release.yml` then fails closed unless all of the following hold on the tag commit itself:

- `vX.Y.Z` equals the Cargo package version and a dated changelog section exists;
- the tag commit equals the live `main` tip and GitHub reports `main` protected;
- locked repository verification, tests, Clippy, rustdoc, statement coverage, and branch coverage succeed;
- a dedicated no-production-secret runner proves rootless Podman, seccomp, SELinux, effective isolation, deny-by-default egress, loopback publication, resource limits, and complete cleanup on the same source SHA;
- two clean checkouts produce byte-identical Cargo source packages;
- the packaged source has an SPDX 3.0.1 SBOM and SHA-256 checksums;
- `release-evidence.json` records source, package, runtime-backend, workflow, and artifact identity;
- GitHub artifact attestations bind build provenance and the SPDX SBOM to the exact Cargo package bytes.

Only after those jobs succeed does the workflow create the GitHub Release. No crates.io or other registry publication is implied by the GitHub Release. A registry adapter may be added only when its credential, package-namespace, provenance, rollback, and ownership policy is explicitly approved.

## Hostile-runner boundary

The release/security runner is not a developer workstation and must not carry production provider credentials. The job receives only the minimum GitHub contents-read token needed to fetch the reviewed source. The immutable OCI fixture is pre-pulled by digest outside the application launch path. Failure, cancellation, or incomplete runner assignment is non-passing release evidence.

The Ubuntu-hosted Podman lane remains useful negative compatibility evidence when the host cannot prove an LSM. It must not be relabeled as successful P0 isolation. Positive first-release evidence requires the dedicated LSM-capable lane or a separately accepted stronger backend.

## Evidence retention and consumer handoff

The release assets include the Cargo package, SPDX SBOM, `SHA256SUMS`, and `release-evidence.json`. GitHub attestations provide the signed provenance/SBOM verification surface. The release tag and GitHub Release remain labels and distribution metadata; consumers should additionally verify artifact checksum and attestation identity before pinning the immutable artifact.

After a protected release:

- refresh Wardnet and contextual-orchestrator integration issues;
- record the released version, source SHA, artifact SHA-256, schema/profile versions, and verification evidence;
- require consumers to pin the released artifact/digest through their Anti-Corruption Layers;
- project only runtime/backend technology, lifecycle, ownership, remediation, architecture-risk context, and provenance through a released compatible Context Graph contract; never promote malware verdicts or artifact risk scores into authoritative EA facts.

## Rollback

A release is immutable. Do not retag or replace release assets under an existing version. If a release defect is discovered, publish a new patch version after the same gates pass. Consumers roll back by pinning the previous verified release artifact and contract version. Runtime-owned ephemeral containers and networks must still be reclaimed before a rollback is reported successful.
