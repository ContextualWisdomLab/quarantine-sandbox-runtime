# ADR 0003: Published-contract consumption and MSA 따로 또 같이

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decision owners:** Quarantine Sandbox Runtime maintainers
- **Scope:** How a host consumes this leaf without sibling checkouts, and
  how naruon and gyeot remain composition hubs

## Context

CWL MSA is **따로 또 같이**: a leaf must run independently and remain
callable by a hub. “따로” fails if this repository cannot be understood
or pinned without cloning naruon, wardnet, gyeot, or EgressWeave. “같이”
fails if the only integration path is to fold the leaf into a hub
repository or to require those hubs as sibling working trees.

Naruon is the email-workspace platform and an allowed composition hub
([ContextualWisdomLab/naruon](https://github.com/ContextualWisdomLab/naruon)).
Gyeot (곁) is the on-device wellness composition hub
([ContextualWisdomLab/gyeot](https://github.com/ContextualWisdomLab/gyeot)).
Those links stay. They are callers, not parents.

NIST places forensic examination and analysis after data have been
acquired (Kent et al., 2006, §§3.1–3.3) and places incident-response
outcomes in the consuming organization’s risk-management program (Nelson
et al., 2025). A published contract is how this leaf reports analysis
evidence without taking that program over.

## Decision

1. **This repository is the contract publisher.** Hosts consume
   [`docs/contracts/consumer-contract.md`](../contracts/consumer-contract.md)
   and the ADRs in this tree. They pin a git revision, a release tag, or
   a later package published from this same product. They do not read
   unpublished files from naruon, wardnet, gyeot, or EgressWeave to learn
   this product’s contract.
2. **No sibling checkout is required.** Building, documenting, or calling
   this leaf must not require `../naruon`, `../wardnet`, `../gyeot`, or
   `../EgressWeave` on disk.
3. **Hubs call; they do not absorb.** Naruon and gyeot may depend on this
   leaf as a published dependency. Wardnet may do the same as a SOC
   consumer. None of those hubs becomes the home of this product. This
   leaf is not merged into naruon, wardnet, or a hub monorepo.
4. **The call shape is submit-bytes-plus-bounded-context, receive
   evidence.** The host already holds the artifact (International
   Organization for Standardization, 2012, identification through
   preservation remain host activities). The leaf does not receive
   source credentials.
5. **Machine-readable schemas, when they exist, are published from this
   repository.** They are not invented on the default branch while the
   product is documentation-only (see ADR 0004). A later implementation
   in this product adds versioned schemas beside the narrative contract.

## Consequences

A host can integrate from one pin of this repository. Hub wiring lives in
the hub’s own pull request. This leaf can be sold, reviewed, and operated
without granting it hub source trees.

Hubs that currently use git submodules for other leaves may later add
this leaf the same way. That is optional consumption of this product, not
a requirement that this product check those hubs out.

If a change makes standalone use depend on a sibling hub checkout, or
moves this product’s source of truth into naruon, wardnet, or another
hub, the MSA contract has been broken.

## References

International Organization for Standardization. (2012). *Information
technology — Security techniques — Guidelines for identification,
collection, acquisition and preservation of digital evidence* (ISO/IEC
27037:2012). https://www.iso.org/standard/44381.html

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
