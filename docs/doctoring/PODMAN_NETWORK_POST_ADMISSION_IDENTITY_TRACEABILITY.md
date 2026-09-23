# Podman post-admission network identity traceability

## Current authority

Draft #142 remains the focused dependent witness for continuity of immutable network authority after admission. The branch is still based on historical #127 exact `e9ad58ceaec006b11da63772c22447b3a424da23`; it must not be treated as dependency-safe merely because GitHub can compute a merge.

Live canonical parent #127 has advanced to exact `5db1275d51790188ab41726b7117f45b283eb5d8`, native CI `35830350461`. Current #127 is Draft/open/mergeable and remains the sole implementation owner for application-service network identity/lifecycle. No parent result transfers to this child.

## Historical RED and what changed upstream

This child was created for a real defect on the historical parent. After a canonical backend network ID had been selected and used for container creation, `verify_effective_isolation()` did not receive that ID. Its later P0 network-state check re-resolved the public `qsr-net-*` correlation. A same-name replacement could therefore supply `internal=true` / DNS-disabled evidence for a different object.

Test-only commit `de78cac3add817e4d2bd24316d8f52676128638f` adds `tests/podman_application_service_network_post_admission_rebind_red.rs`. The fake backend lets the historical flow select `OWNED_NETWORK_ID`, then rebinds the public correlation after container start while exact-ID inspection of the owned network remains available. The witness requires later P0 verification to stay bound to the immutable authority used for container creation.

Live #127 production ancestry has since implemented this invariant. Executed parent evidence exposed the effective-attachment defect, and production repair `6629fd30fa7c011f5aaff1b19ec1b116db2696bd`:

- carries the admitted network ID into `verify_effective_isolation()`;
- deserializes effective `NetworkSettings.Networks` and requires exactly one attachment whose `NetworkID` equals that admitted ID;
- performs the later network-state inspection by the same exact ID instead of `plan.network_name()`;
- uses exact-ID, non-force network removal on partial-launch cleanup after network-ID admission.

The historical statement that live parent production still drops the network ID at the verification boundary is therefore obsolete. The #142 invariant is present in current #127 production code, but #142 is not yet succeeded: its executable witness and branch ancestry still describe the pre-repair call shape and current #127 has not reached one unchanged dependency-safe exact-head GREEN.

## Current #127 creation-bound identity gate

The current parent has moved beyond the old name-inspect acquisition model. Production `0e44b88b7ecf72a5c641b050a7d8450d604da4e4`, corrected by `421b8913cd4f74a0728a138f856409de1700fcb6`, captures bounds around non-`--ignore` `podman network create`, queries bounded non-streaming `podman events` for the creation event, admits exactly one generated correlation plus canonical 64-lower-hex backend ID, verifies P0 state by that exact ID, binds container creation to it, and preserves exact-ID/non-force partial cleanup. Missing, malformed, ambiguous, disabled, or lost receipt history remains fail closed; the public correlation is not a destructive fallback.

Exact `2f053b37a3ae52dcd4d102cd99e67736e5f28dd9`, CI `35817422499`, acquired hosted runners. The broad process-boundary fixture failed first: 8 of 9 `tests/podman_application_service.rs` cases stopped at `network_creation_receipt` because that fake backend had not been migrated to create → receipt → exact-ID P0 sequencing. That execution did not prove the dedicated Podman-v6 JSON-key mismatch.

Current test-only parent `5db1275d51790188ab41726b7117f45b283eb5d8` migrates only that broad fixture. It intentionally emits uppercase `Network` so production can cross the prerequisite without pre-applying the next repair. Production Rust/API/schema are unchanged by this current head movement.

The next transport contract remains explicit: Podman v6 `--format json` emits `{ID, network, Status, Type}` with lowercase `network`, while current QSR production still maps uppercase `Network`. The dedicated witness must become causal before the serde mapping changes.

After that gate, current #127 still has independent obligations for missing `EffectiveCaps`, missing `BoundingCaps`, missing post-start `dns_enabled`, no-admitted-ID recovery (#141), successful-lease private exact network cleanup authority, foreign-safe non-force explicit termination, and remote Podman/Podman-machine/Colima clock-domain parity.

## Security invariant retained by #142

Once one creation-bound, provenance-admitted immutable network authority exists, every later security or destructive operation must continue to use that same ID or an equivalent private capability derived from it. Public `qsr-net-*` may remain consumer-visible correlation/audit metadata, but it must not regain authority for:

- effective attachment verification;
- post-start P0 `internal` / DNS state verification;
- partial-launch cleanup;
- successful-lease private cleanup;
- explicit termination;
- recovery deletion.

A later network inspection may corroborate current state only when it addresses the admitted immutable object. Matching configuration reached through a mutable name is not ownership evidence.

## Required ordinary/non-force succession

Do not merge the historical #142 branch merely because current #127 production appears to satisfy the invariant. Complete succession requires the child evidence to be adapted and executed on the dependency-safe parent.

After #127 becomes dependency-safe:

1. ordinary/non-force adopt the exact verified parent while preserving every #142 delta, fixture, contract, test, and traceability item;
2. adapt the witness from historical name-based acquisition to the current create → event receipt → exact-ID P0 path;
3. retain the hostile same-name replacement after admission and require that it cannot influence effective attachment, post-start P0 state, or partial cleanup;
4. preserve public correlation/private authority separation even if the exact backend call shape changes;
5. execute the adapted child exact independently;
6. only when that exact is GREEN and all valid #142 evidence is demonstrably inherited may the PR be considered succeeded, merged, or closed under the PR-zero rule.

No source copy, force push, destructive rebase, obsolete-call-shape preservation, predecessor-GREEN transfer, or public-name fallback is authorized.

## Decision record

**Problem.** Historical parent code could select an immutable network ID for container creation and later discard that authority by re-resolving a public correlation for security evidence.

**Constraint.** Preserve the consumer-visible correlation, exact-container identity, fail-closed isolation, DDD ownership, exact-ID partial cleanup, and foreign-safe lifecycle semantics while allowing the implementation mechanism to evolve.

**Rejected alternatives.** Rechecking the generated name, increasing name entropy, or requiring only `internal=true` / DNS-disabled state do not establish object continuity. Canonical ID syntax alone is also insufficient unless the ID is creation-bound and provenance-admitted.

**Selected direction.** Establish one creation-bound network authority at #127, then retain that same private authority through effective verification and all later lifecycle operations. Adapt #142 to current parent mechanics rather than preserving obsolete CLI sequencing.

**Effect.** A same-name replacement cannot become an evidence source or cleanup target after immutable admission, while the child remains a reproducible hostile witness instead of a stale implementation snapshot.

## References

Podman Authors. (2026). *podman-network-create — Create a Podman network* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-create.1.html

Podman Authors. (2026). *podman-network-inspect — Display the network configuration for one or more networks* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-inspect.1.html

Podman Authors. (2026). *podman-network-rm — Remove one or more networks* (Podman 6.0.0). Podman documentation. https://docs.podman.io/en/v6.0.0/markdown/podman-network-rm.1.html

Souppaya, M. P., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190
