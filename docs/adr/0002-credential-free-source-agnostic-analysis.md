# ADR 0002: Credential-free, source-agnostic quarantine analysis

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decision owners:** Quarantine Sandbox Runtime maintainers
- **Scope:** Why this leaf analyzes already-held artifact bytes without
  source-system credentials or source-specific fetchers

## Context

Hostile artifacts arrive through many channels. A GitHub comment, an
email attachment, a connector upload, and a direct API submit can carry
the same bytes. Binding analysis to one source’s API, bot, or credential
would make the product a GitHub-only or mailbox-only tool and would
expand its privilege beyond the artifact.

NIST SP 800-86 treats forensics as an IT incident-response capability
that works across data sources—files, operating systems, network traffic,
and applications—rather than as a source-specific collector (Kent et al.,
2006, Abstract; ch. 3–4). Chapter 3 of that guide separates the forensic
process into data collection, examination, analysis, and reporting, and
treats collection as identifying possible sources and acquiring data
before examination (Kent et al., 2006, §§3.1–3.4). File integrity during
collection is an explicit concern (Kent et al., 2006, §4.2.2).

ISO/IEC 27037:2012 is a published International Standard for
identification, collection, acquisition, and preservation of potential
digital evidence (International Organization for Standardization, 2012).
The ISO catalog, checked on 18 August 2026, records a systematic review
opened on 15 July 2023 and closed on 3 December 2023, and lists the current
stage as 90.60, “Close of review.” The lifecycle page also identifies 90.92,
“International Standard to be revised,” as a possible review outcome, but the
catalog does not show this edition at 90.92. This ADR therefore does not rely
on the earlier 2018 confirmation or assert a post-2023 confirmation. The four
handling activities remain the host’s responsibility; this leaf is not the
first-custody handler.

NIST SP 800-61 Revision 3, which superseded the withdrawn Revision 2 on
3 April 2025, provides organization-level incident-response guidance as a
CSF 2.0 community profile (Nelson et al., 2025). CSF 2.0 itself describes
broader organization-wide cybersecurity outcomes across Govern, Identify,
Protect, Detect, Respond, and Recover (National Institute of Standards and
Technology, 2024). Analysis evidence can support incident-response outcomes;
it does not replace the organization’s incident program.

A credential-bearing, source-specific design would contradict those
separations: the analyzer would become a collector, an identity broker,
and a policy engine.

## Decision

This product is **credential-free** and **source-agnostic**.

1. The host that already holds the artifact submits the bytes. This leaf
   does not authenticate to the source system and does not accept
   source-system secrets.
2. Source identity is not required for analysis. The host may attach only
   optional `bounded_source_context` that conforms exactly to the closed
   field allowlist, byte limits, prohibited-data rules, logging exclusions,
   and request-lifetime deletion rule in
   [`docs/contracts/consumer-contract.md`](../contracts/consumer-contract.md).
   The runtime rejects unknown metadata and does not fetch, crawl, or replay
   the source.
3. Artifact identity is the submitted bytes. Integrity language in this
   product refers to preserving that submitted representation for
   examination and analysis, not to field collection from live systems.
4. Collection, acquisition, and legal preservation of original evidence
   remain with the host and its incident process (International
   Organization for Standardization, 2012; Kent et al., 2006, §3.1).
5. Examination and analysis of the submitted artifact, and reporting of
   analysis evidence back to the host, are this leaf’s intended forensic
   contribution (Kent et al., 2006, §§3.2–3.4).
6. Incident handling outcomes remain with the host’s program (Nelson et al.,
   2025).

## Consequences

Any authorized host can call the same contract without granting this leaf
mailbox, forge, or object-store credentials. A compromise of the runtime
does not yield source-system keys it never held.

Hosts must perform collection and admission themselves. This leaf cannot
prove that an email was quarantined before store, that a GitHub comment
was deleted, or that a WAF rule fired.

NIST SP 800-86 is written from an IT view, not a law-enforcement view,
and is not legal advice (Kent et al., 2006, Abstract). Operators remain
responsible for local law, retention, and counsel.

ISO/IEC 27037 is a paid International Standard. This ADR cites the
official ISO catalog record and does not reproduce the standard’s text.

## References

International Organization for Standardization. (2012). *Information
technology — Security techniques — Guidelines for identification,
collection, acquisition and preservation of digital evidence* (ISO/IEC
27037:2012). Retrieved August 18, 2026, from
https://www.iso.org/standard/44381.html

Kent, K., Chevalier, S., Grance, T., & Dang, H. (2006). *Guide to
integrating forensic techniques into incident response* (NIST Special
Publication 800-86). National Institute of Standards and Technology.
https://doi.org/10.6028/NIST.SP.800-86

National Institute of Standards and Technology. (2024). *The NIST
Cybersecurity Framework (CSF) 2.0* (NIST CSWP 29).
https://doi.org/10.6028/NIST.CSWP.29

Nelson, A., Rekhi, S., Souppaya, M., & Scarfone, K. (2025). *Incident
response recommendations and considerations for cybersecurity risk
management: A CSF 2.0 community profile* (NIST Special Publication
800-61r3). National Institute of Standards and Technology.
https://doi.org/10.6028/NIST.SP.800-61r3
