# ADR 0001: Product and authority boundary

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decision owners:** Quarantine Sandbox Runtime maintainers
- **Scope:** What this leaf owns versus wardnet WAF/IDS, naruon email
  admission, and EgressWeave outbound HTTP

## Context

ContextualWisdomLab sells and operates several security-adjacent products.
Mixing their authorities into one repository would hide privilege, force
sibling checkouts, and make standalone use depend on a hub.

Wardnet is the WAF/IDS and AI SOC edge
([ContextualWisdomLab/wardnet](https://github.com/ContextualWisdomLab/wardnet)).
Naruon is the email workspace that decides whether inbound mail and
attachments enter its store
([ContextualWisdomLab/naruon](https://github.com/ContextualWisdomLab/naruon)).
EgressWeave is SSRF- and DNS-rebinding-safe outbound HTTP for Python
([ContextualWisdomLab/EgressWeave](https://github.com/ContextualWisdomLab/EgressWeave)).

NIST describes forensics as a capability that supports incident response
rather than replacing the incident-response program (Kent et al., 2006).
The current NIST incident-response profile places preparation, detection,
response, and recovery in the organization’s cybersecurity risk-management
activities (Nelson et al., 2025; National Institute of Standards and
Technology, 2024). ISO/IEC 27037 assigns identification, collection,
acquisition, and preservation of potential digital evidence to the
handling process that precedes later analysis (International Organization
for Standardization, 2012).

This leaf must therefore own analysis evidence for artifacts a host
already holds, and must not absorb edge enforcement, mailbox admission, or
outbound HTTP.

## Decision

Quarantine Sandbox Runtime owns **source-agnostic, credential-free
artifact-analysis evidence**. It accepts hostile artifact bytes and, only when
needed, optional `bounded_source_context` that conforms exactly to the closed
allowlist, byte limits, prohibited-data rules, logging exclusions, and
request-lifetime deletion rule in
[`docs/contracts/consumer-contract.md`](../contracts/consumer-contract.md).
No other source metadata is part of the public contract. It does not fetch the
source system, hold source credentials, decide malicious or benign status,
admit email, enforce WAF/IDS policy, or open outbound HTTP.

Neighbor authority stays in the neighbor product:

| Authority | This runtime | Wardnet | Naruon | EgressWeave |
| --- | --- | --- | --- | --- |
| Artifact-analysis evidence | Owns | May consume | May consume | Does not own |
| WAF/IDS and edge enforcement | Does not own | Owns | Does not own | Does not own |
| Email admission and mailbox store | Does not own | Does not own | Owns | Does not own |
| Malicious/benign verdict and SOC response | Does not own | Owns SOC policy | Owns local admission policy | Does not own |
| Incident, block, notify, retain | Does not own | Owns SOC response | Owns email-workspace response | Does not own |
| Outbound HTTP / SSRF-safe egress | Does not own | Host-local if any | Uses EgressWeave where Python egress is required | Owns |
| Source-system credentials | Never held | Host/SOC credentials | Email and connector credentials | Optional TLS client identity for an already-allowed authority |

Naruon and gyeot remain allowed composition hubs. They call this leaf;
this leaf is not folded into them. Wardnet may call the same published
contract as a SOC consumer without moving this product into the wardnet
repository.

## Consequences

Buyers can operate or embed this leaf without taking a WAF, an email
workspace, or an outbound HTTP library. Hosts keep least privilege: they
submit bytes they already hold and keep verdict, admission, and egress
policy local. Optional source context is minimized without altering or masking
the artifact bytes that the host has authorized for analysis.

End-to-end properties such as “the attachment never entered the mailbox”
or “the HTTP request was blocked” cannot be proven by this repository
alone. Those claims require host verification in naruon, wardnet, or the
caller that owns the corresponding authority.

If a change in this repository begins to score HTTP transactions, admit
mail, store source credentials, accept arbitrary source metadata, or open
unconstrained outbound HTTP, the authority contract has been broken and the
expansion must be reverted.

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
