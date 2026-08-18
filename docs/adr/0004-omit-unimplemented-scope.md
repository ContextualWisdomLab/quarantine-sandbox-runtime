# ADR 0004: Omit unimplemented product scope

- **Status:** Accepted
- **Date:** 2026-08-18
- **Decision owners:** Quarantine Sandbox Runtime maintainers
- **Scope:** What this default-branch publication must not invent or claim

## Context

The default branch of this repository contains product documentation and
contracts. It does not contain an executable runtime, a language package,
or a test harness.

NIST SP 800-86 warns that its forensic guidance is not an all-inclusive
step-by-step investigation manual and is not legal advice (Kent et al.,
2006, Abstract). NIST SP 800-61 Revision 3 and CSF 2.0 describe
organization-level incident-response outcomes, not a single vendor
sandbox (Nelson et al., 2025; National Institute of Standards and
Technology, 2024). ISO/IEC 27037 covers identification through
preservation of potential digital evidence, not a named analysis engine
(International Organization for Standardization, 2012).

Documenting those standards does not create a shipped analyzer, a
package name, or a CI harness. Claiming those artifacts on a
documentation-only branch would invent product surface that buyers cannot
run.

## Decision

This default-branch publication **omits** any feature that is not
actually in this product’s published scope.

It does **not** invent or claim:

- an executable runtime, service binary, container image, or local
  develop-server command
- a language package (crate, wheel, npm library) or install command
- a test harness, coverage gate, or smoke script
- a machine-readable OpenAPI or JSON Schema that does not exist in this
  tree
- static-analyzer adapters, dynamic detonation, reputation look-ups, or
  LLM verdict reasoning
- WAF/IDS enforcement, email admission, outbound HTTP, incident
  retention, or source-system credential brokerage

Those omissions are intentional. A later implementation, if it lands in
**this** repository, publishes its own package, schemas, and
verification from this product. Until then, the operator surface is the
README, the ADRs, and
[`docs/contracts/consumer-contract.md`](../contracts/consumer-contract.md).

Standards citations justify the authority split. They do not imply that
this repository currently performs collection, examination, analysis, or
reporting in software.

## Consequences

Buyers and hosts can trust the default branch as a contract, not as a
silent runtime. Reviewers do not have to hunt for a package that was
never published.

The cost is that independent use today is contract adoption, not
`install` and `run`. That is accurate. Inventing a fake harness would be
worse: hosts would couple to an API that this branch cannot honor.

If a future change adds a runtime, package, or harness, it must appear as
real files in this repository and must not be described as already
shipping before those files exist.

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
