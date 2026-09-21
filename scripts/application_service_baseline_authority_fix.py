#!/usr/bin/env python3
"""One-shot correction for the application-service integration baseline authority.

The owner integration fixer prepends the baseline marker before production repair. This follow-up
replaces only that newly-created authority section so the durable ledger distinguishes real causal
REDs from capability controls that are already GREEN on the command-runtime lineage. The source-fix
workflow removes this script before publishing the repaired descendant.
"""

from pathlib import Path


BASELINE_PATH = Path("docs/product-technical-gap-baseline.md")
MARKER = "<!-- current-authority-2026-09-14-application-service-owner-integration -->"
SECTION_END = "\n---\n"

baseline = BASELINE_PATH.read_text()
start = baseline.find(MARKER)
if start < 0:
    raise SystemExit("application-service owner integration authority marker is missing")
end = baseline.find(SECTION_END, start)
if end < 0:
    raise SystemExit("application-service owner integration authority section is unterminated")
end += len(SECTION_END)

authority = f'''{MARKER}
# Current authority supersession — application-service owner integration (2026-09-14)

Draft #112 exact `cc82dac196d133cb614aebc2ab7f37be945f48d1` keeps bounded command execution in the `sandbox_execution` Core context and the application-service profile in its Supporting context. Draft #118 is the causal owner-adoption lane, while #117 remains the ordinary ancestry-reconciliation lane for canonical #21 exact `65f69de6eb1cf78b316b38424f8c35c316cd0672`; reimplementation alone does not close ancestry inheritance.

The #118 source-fix admission deliberately separates seven real causal REDs from five already-satisfied capability preservation controls. Error ownership, runtime identity uniqueness, duplicate network evidence, malformed create receipt, missing create receipt, exact post-create container ownership, and forged/deserialized lease cleanup authority must each be RED before mutation. CAPEFF/CAPBND/CAPINH/CAPPRM/CAPAMB are executed as five independent exact tests and must remain GREEN before mutation because the current command-runtime production parser already retains and checks all five columns. Manufacturing nonzero results for already-correct capability handling is not accepted as RED evidence.

The repair adopts only the missing owner semantics: each service create receives a fresh runtime identity and runtime-owned cidfile receipt; destructive post-create container operations use the exact acquired container ID; generated `qsr-app-*` stays correlation metadata; cleanup authority is crate-private and non-serializable; forged/deserialized public lease evidence cannot recreate destructive authority; cleanup receipts identify the exact container actually removed. Caller-visible serialized lease receipts are compared only on their versioned public evidence when authorizing a caller-owned registry entry, while the coordinator always passes its registered in-process lease with private cleanup authority to the backend. This preserves legitimate transport round trips without allowing deserialization to mint cleanup capability. `network_id` remains the versioned `qsr-net-*` lease correlation identifier until canonical #23/#48 acquires and binds the Podman network `.ID`; #118 therefore does not claim exact network lifecycle ownership.

The one-shot workflow verifies its exact checkout against `$GITHUB_SHA`, rejects unrelated `backend_security_info` failures, proves focused post-repair GREEN, removes every source-fix script and itself, then requires repository validation, full workspace/all-target tests, Clippy `-D warnings`, public/private rustdoc `-D warnings`, rustfmt, and `git diff --check` before an ordinary non-force publication. The post-create lifecycle witness now crosses the public JSON lease boundary through `ApplicationServiceCoordinator`, so it also protects the distinction between public receipt equality and private destructive authority. No predecessor GREEN transfers to the resulting candidate. Native exact-head CI, complete owned-production coverage, dedicated positive effective-LSM, #35/#43 real-runtime enforcement, qualifying independent review/security, #117 ancestry reconciliation, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback remain mandatory.

---
'''

BASELINE_PATH.write_text(baseline[:start] + authority + baseline[end:])
