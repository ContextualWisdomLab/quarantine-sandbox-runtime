#!/usr/bin/env python3
"""Synchronize the Unreleased changelog with current command-runtime and CI authority."""

from pathlib import Path

path = Path("CHANGELOG.md")
text = path.read_text(encoding="utf-8")

stale_ci = "- Repository authority is the protected/default `develop` branch. Native CI still has a stale `main`-only push trigger; issue #24 carries the focused RED requiring exact post-integration `develop` evidence before release authority. The release preflight likewise remains Draft behind its protected-default-source RED and must not treat `main` as authoritative while the repository default is `develop`."
current_ci = "- Repository authority is the protected/default `develop` branch. Native CI now triggers on pushes to `develop`; issue #24's causal trigger repair is retained on the active owner lineage. Release authority still requires native CI to execute on the exact protected integration SHA, and the Draft release preflight must verify that same protected source rather than a mutable PR head."
if text.count(stale_ci) != 1:
    raise SystemExit("stale native-CI changelog authority not found exactly once")
text = text.replace(stale_ci, current_ci, 1)

changed_anchor = "- Production consumers are required to pin a released package checksum and provenance/SBOM evidence rather than a transient pull-request head or branch artifact."
changed_add = "- The one-shot command runtime now acquires the exact created container identity, runs `podman init`, and rejects contradictory static/configured isolation while the payload is still initialized-but-not-started before invoking `podman start`; live process evidence remains a separate post-start verifier. This is a bounded partial repair for issue #25, not effective pre-payload attestation or release evidence."
if text.count(changed_anchor) != 1:
    raise SystemExit("Changed-section anchor missing or ambiguous")
if changed_add not in text:
    text = text.replace(changed_anchor, changed_anchor + "\n" + changed_add, 1)

security_anchor = "- Effective isolation is inspected after container start; read-only rootfs, no-new-privileges, private namespaces, seccomp, an LSM, resource limits, internal networking, and loopback publication must be positively verified for the P0 lease path."
security_add = "- Command execution additionally fails closed when `podman init` is unavailable or initialized/static configuration already contradicts the command isolation profile before `start`. Initialized/static state is never promoted to effective seccomp/LSM/capability proof; issue #25 remains open until a runtime-owned effective hold/attest/release boundary proves those controls before hostile consumer release."
if text.count(security_anchor) != 1:
    raise SystemExit("Security-section isolation anchor missing or ambiguous")
if security_add not in text:
    text = text.replace(security_anchor, security_anchor + "\n" + security_add, 1)

release_anchor = "- No protected product release exists yet. `0.1.0` remains the package version under development until the complete stacked runtime integrates and a dated changelog section is reviewed on the exact protected stable candidate."
release_add = "- Command-runtime issue #25 is still a P0 release blocker. The executed hostile payload-side-effect RED proves post-start attestation is too late; the current `podman init` hold/configuration gate is only partial progress. Same-head runtime-owned effective hold/attest/release GREEN and real positive-LSM acceptance are required before command execution can be called release-ready."
if text.count(release_anchor) != 1:
    raise SystemExit("Not-yet-release anchor missing or ambiguous")
if release_add not in text:
    text = text.replace(release_anchor, release_anchor + "\n" + release_add, 1)

path.write_text(text, encoding="utf-8")
