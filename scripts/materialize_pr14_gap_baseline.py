#!/usr/bin/env python3
"""Materialize the current PR14 init-hold authority into the product/technical gap baseline."""

from pathlib import Path

path = Path("docs/product-technical-gap-baseline.md")
text = path.read_text(encoding="utf-8")

section_start = "### Current exact command-runtime authority\n\n"
section_end = "\n\n## Product responsibility and DDD"
if text.count(section_start) != 1 or text.count(section_end) != 1:
    raise SystemExit("command-runtime authority section shape changed; refusing ambiguous update")
start = text.index(section_start) + len(section_start)
end = text.index(section_end, start)
current_authority = """PR #14 first converted issue #25 into an executed causal P0 on exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`: verify `102428599037` and branch coverage `102428599228` both reached `podman_command_execution_pre_attestation_red::command_payload_is_not_runnable_before_effective_process_attestation` after the typed process-boundary and preceding command-runtime/application-service suites passed. The hostile fixture observed the consumer payload side effect after `podman start` and before effective process attestation failed. Hosted negative rootless/AppArmor `102428599461` was GREEN; dedicated positive-LSM was not acceptance evidence. Cleanup after that point could not undo payload execution.\n\nThe next owner-path RED narrowed a useful earlier hold point without closing the P0. Test-bearing `9e36766abc0a553f45c62fa327b34f1315dc0db9`, executed after immutable-fixture prerequisites on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / native CI `34353894509`, showed that production never invoked `podman init`: an unavailable init primitive did not fail closed, invalid configured isolation was not rejected while the container was still held, and both cases reached `start`/payload side effects. Minimum production `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` plus formatter-only `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e` now performs `create -> acquire exact ID -> podman init -> pre-start static/configuration verification -> start -> live process verification`. Static checks are shared pre/post start but remain configured-state evidence, not effective seccomp/LSM/capability proof.\n\nSubsequent fixture work made the immutable fake backend init-capable and repaired the deterministic start-failure fixture so its intended `container_start` error is reached after valid pre-start inspection. The one-purpose source materializer that applied that fixture repair has been removed after the source delta was materialized. The current owner therefore contains the repaired test without retaining a self-modifying workflow. Proposed ADR-0008 and `docs/doctoring/COMMAND_PRE_ATTESTATION_TRACEABILITY.md` now record the init-held partial boundary and its limit. Full #25 remains open: `podman init` plus static inspection cannot prove effective consumer-process seccomp/LSM/capability state before exec, so the next RED must prove a runtime-owned effective hold/attest/release primitive and real rootless positive-LSM acceptance before any release claim."""
text = text[:start] + current_authority + text[end:]

lines = text.splitlines()
replacement_25 = "- #25 pre-attestation execution — original RED `585f3d955bddfb95f28e5918cfcadcac632589df` executed causally on exact `ed9318ba96d876341866d84a892d0145e12f6469`, native CI `34340070150`: the hostile fixture proved the consumer became runnable at `podman start` before live attestation. The later init-hold RED `9e36766abc0a553f45c62fa327b34f1315dc0db9`, executed on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / CI `34353894509`, then proved production lacked an earlier `podman init` hold and could not reject invalid configured isolation before start. Minimum source `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` + formatter `f1a037931ecc7fcbd6e87836f8fb7048c2c3544e` now fails closed on init failure and verifies static/configuration controls while the exact acquired container remains initialized but not started. This is necessary partial GREEN only: it does not prove effective seccomp/LSM/capability state at consumer exec. Full GREEN still requires a runtime-owned effective hold/attest/release primitive, bounded evidence/release channels, the original payload-side-effect regression GREEN on the same implementation, and real rootless positive-LSM E2E. See `docs/doctoring/COMMAND_PRE_ATTESTATION_TRACEABILITY.md` and ADR-0008."
count_25 = 0
for index, line in enumerate(lines):
    if line.startswith("- #25 pre-attestation execution —"):
        lines[index] = replacement_25
        count_25 += 1
if count_25 != 1:
    raise SystemExit(f"expected one #25 baseline row, found {count_25}")
text = "\n".join(lines) + "\n"

verification_anchor = "This evidence is intentionally preserved as RED; no predecessor or sibling GREEN can waive it."
verification_addition = "\n\nThe same canonical owner subsequently executed the `podman init` hold-capability RED on exact `eb2330ffb6689b07706a305e55c81871414e9fe5` / CI `34353894509` and adopted the minimum `40313a2a8fe058f3cb25d3580b4f1fecbb4ec2e1` repair. The owner now rejects init unavailability and invalid configured isolation before start, while retaining post-start live attestation. This advances #25 but deliberately does not convert it to integrated GREEN: effective-process proof still occurs after `start`, and the required runtime-owned hold/attest/release plus positive-LSM acceptance remains outstanding. The deterministic start-failure fixture and formatter drift exposed by the new order were repaired without weakening production, and the temporary source-materialization workflow/script were removed after their source delta was committed."
if verification_anchor not in text:
    raise SystemExit("verification anchor missing")
if verification_addition.strip() not in text:
    text = text.replace(verification_anchor, verification_anchor + verification_addition, 1)

next_lines = text.splitlines()
next_1 = "1. Starting from the current `podman init` held state, make issue #25's next effective capability RED executable on canonical #14: prove a supported Podman/OCI primitive can keep consumer execution non-runnable while a runtime-owned immutable attestor runs in the required effective context, exposes bounded authoritative evidence, and releases only after positive policy evaluation. Fail closed when the primitive, attestor identity, channel, or required evidence is unavailable/malformed. Do not relabel init/static inspection as effective proof or substitute plain `podman exec`."
next_2 = "2. Only after that effective hold/attest/release RED executes for the intended cause, implement the smallest runtime-owned adapter and require the original `podman_command_execution_pre_attestation_red` plus the init-hold regressions to be GREEN on the same implementation. Real rootless positive-LSM E2E remains independent acceptance evidence."
seen_1 = seen_2 = 0
for index, line in enumerate(next_lines):
    if line.startswith("1. Make issue #25's next capability RED executable"):
        next_lines[index] = next_1
        seen_1 += 1
    elif line.startswith("2. Only after that capability RED executes"):
        next_lines[index] = next_2
        seen_2 += 1
if seen_1 != 1 or seen_2 != 1:
    raise SystemExit(f"next-slice shape changed; found item1={seen_1}, item2={seen_2}")

path.write_text("\n".join(next_lines) + "\n", encoding="utf-8")
