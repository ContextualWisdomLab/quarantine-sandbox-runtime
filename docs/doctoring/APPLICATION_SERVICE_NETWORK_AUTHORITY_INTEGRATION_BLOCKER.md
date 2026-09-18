# Application-service network authority integration blocker

Date: 2026-09-14

## Finding

PR #118 exact `fadf62cea182407833513dcf6ee2df485a522faa` must not publish its current one-shot repair candidate.

`application_service_owner_integration_fix.py` constructs the runtime-private `ApplicationServiceCleanupAuthority.network_id` from `RuntimeLeaseMetadata.network_id`. The same repair populates that metadata with `plan.network_name()`, the generated `qsr-net-*` correlation name. Its repaired `terminate_at` then executes `podman network rm --force <authority.network_id>`.

That makes a caller-invisible private capability look safer without changing the underlying selector: destructive network authority still comes from a re-resolvable correlation name.

## Canonical owner conflict

PR #23 / issue #48 owns acquired Podman network identity. Its checked-in RED requires the runtime to inspect the network immediately after creation, acquire the full Podman `.ID`, and bind container creation to that ID before a same-name replacement can be selected. Public `qsr-net-*` remains correlation metadata.

PR #23 / issue #41 separately owns foreign-safe cleanup. Its RED demonstrates that `podman network rm --force` can broaden deletion authority to foreign containers attached to the network; cleanup must not use network-level force removal as a substitute for ownership proof.

Therefore #118 cannot claim exact network lifecycle authority, and it cannot use the correlation name as the private destructive selector. Exact acquired container-ID work in #118 remains valid but is insufficient to close the network boundary.

## Required integration order

1. Reacquire causal RED evidence for #23/#41/#48 on a rustfmt-clean exact head.
2. Repair the canonical network owner so launch acquires the created network `.ID` before container creation, binds the container to that acquired ID, verifies exclusive deny-by-default attachment, and performs foreign-safe non-force cleanup against acquired authority.
3. Validate focused and full exact-head GREEN, including name-rebind and foreign-member hostile cases.
4. Integrate that canonical network-owner ancestry into the application-service stack without source copying or force rewrite.
5. Only then adapt/re-run #118's application-service owner source-fix and reacquire all exact-head RED/GREEN/full-validation evidence.

## Publication safety consequence

This documentation commit deliberately advances the #118 branch beyond `fadf62c...`. The existing source-fix publisher already requires the live branch head to equal its triggering `GITHUB_SHA` immediately before commit and push. Consequently, any queued `fadf62c...` run must fail closed at publication even if runner admission later resumes.

No GREEN, merge, release, or #23/#48 completion is claimed here. This is a dependency and evidence correction that prevents a stale candidate from acquiring write authority.
