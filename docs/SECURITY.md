# Security

## Security promise

The foundation runtime parses bounded metadata and bytes without executing the artifact, making outbound requests, or receiving credentials. It returns evidence and explicit limitations rather than claiming an unsupported safety verdict.

## Input handling

- Reject empty artifacts.
- Enforce byte and name limits before cloning input.
- Reject control characters in identity-bearing metadata.
- Compute SHA-256 before analyzer invocation.
- Preserve original bytes exactly.
- Treat extensions as hints only after binary magic checks.
- Use strict, versioned output contracts.

## Analyzer requirements

A future analyzer adapter must:

- be uniquely identified;
- emit typed findings with bounded attributes;
- expose failures as structured evidence;
- run under a declared resource policy;
- avoid network access unless a specific reviewed profile permits a controlled sinkhole;
- never receive provider, GitHub, cloud, database, or customer credentials;
- declare tool version, ruleset hash, image digest, and source revision.

## Dynamic worker requirements

Dynamic execution is not present in the foundation. A future worker must use disposable isolation, read-only base images, ephemeral writable overlays, host-enforced network policy, CPU/RAM/PID/disk/time limits, and complete destruction after each sample. Windows and Linux pools are separate trust domains.

## Supply chain

- Rust dependencies are locked in `Cargo.lock`.
- GitHub Actions are pinned by commit SHA.
- Releases require SBOM and provenance.
- External analyzer licenses and data licenses must be documented before adoption.
- Security advisories for the runtime, hypervisor, parser, and analyzers are release blockers according to severity policy.

## Reporting

Do not upload real malware samples to public issues, pull requests, CI artifacts, or repository fixtures. Security reports must follow the organization security policy.
