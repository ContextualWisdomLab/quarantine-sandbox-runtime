# Quarantine Sandbox Runtime

**Source-agnostic, credential-free artifact analysis boundary for the ContextualWisdomLab security ecosystem.**

Quarantine Sandbox Runtime is the dedicated isolation boundary for analyzing untrusted artifacts without giving the analysis environment ambient product credentials or source-system authority.

## What this repository owns

This repository owns the **quarantined analysis runtime boundary**: a place where artifacts can eventually be inspected under explicit resource, filesystem, network, and output controls while remaining separated from credential-bearing publishers and authoritative product runtimes.

It does not own vulnerability policy, repository credentials, deployment authority, product data, or the decision to publish or merge an analysis result. Those responsibilities remain with their dedicated control planes.

## Why it exists

Security analysis frequently needs to execute parsers, scanners, or other tooling against content that should be treated as hostile. Mixing that work with credentials or privileged product state increases the blast radius of a compromised artifact or analysis tool.

The intended product boundary therefore emphasizes:

- no ambient repository, provider, or deployment credentials inside the analysis runtime;
- explicit input and output artifacts rather than shared application storage;
- bounded filesystem, process, network, time, and resource authority;
- evidence that can be reviewed by a separate trusted publisher or control plane;
- source-agnostic integration so one product's data model does not become sandbox authority.

These are target responsibilities, not claims that the current repository already implements them.

## Current status

Protected `develop` is currently a **documentation-only bootstrap**. It contains no executable sandbox, container image, package, runtime policy, scanner integration, benchmark, release, or deployment artifact.

There is nothing to install or run yet. Consumers must not infer isolation guarantees from the repository name alone; those guarantees become real only when implementation, tests, and deployment evidence land together.

## Planned integration boundary

```text
Untrusted artifact
       │
       ▼
┌──────────────────────────────┐
│ Quarantine Sandbox Runtime   │
│ credential-free analysis     │
└──────────────┬───────────────┘
               │
        bounded evidence
               │
               ▼
 Trusted reviewer / publisher
```

The trusted caller decides what enters the sandbox and what, if anything, is accepted afterward. Analysis output is evidence, not automatic authority.

## Quality and security posture

The current repository makes no claim of container hardening, syscall isolation, network denial, resource enforcement, vulnerability coverage, test coverage, performance, or production readiness because none of those controls are implemented in the protected tree yet.

When runtime work begins, security claims should be backed by executable tests and deployment evidence for the exact isolation boundary being described. Credential-free operation must remain a design invariant rather than a documentation convention.

## Contributing

Keep untrusted execution and analysis here; keep credentials, merge/release authority, product-specific policy, and authoritative business data outside this boundary. New dependencies and tools must permit commercial use under the intended distribution model and retain all required license and attribution evidence.

Substantive runtime changes should arrive with architecture, threat-model, test, operability, and integration documentation so downstream callers can distinguish implemented guarantees from plans.

## License

Quarantine Sandbox Runtime is licensed under the [Apache License 2.0](LICENSE).
