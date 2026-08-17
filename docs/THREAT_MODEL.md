# Threat Model

## Assets

- Original artifact bytes and cryptographic identity.
- Analysis request context.
- Evidence records and provenance.
- Runtime policy, analyzer versions, rules, and images.
- Consumer trust in the completeness declaration.
- Host, worker pool, CI, and operator credentials.

## Adversaries

- Artifact authors attempting parser exploitation, resource exhaustion, sandbox escape, or false evidence.
- Submitters manipulating filenames, source context, or trigger metadata.
- Compromised analyzer dependencies or rule feeds.
- Tenant actors attempting cross-tenant evidence access.
- Operators or integrations accidentally treating incomplete analysis as benign.

## Foundation threats and controls

| Threat | Control |
|---|---|
| Oversized artifact exhaustion | configured hard byte limit before clone |
| Empty or malformed submissions | fail-closed validation |
| Filename control-character injection | metadata validation |
| Extension spoofing | binary magic precedes text/extension heuristics |
| Silent analyzer crash | attributable `tool_failure` evidence |
| Unsupported dynamic profile reported as success | `inconclusive` plus explicit limitation |
| Credential exposure | runtime manifest requires `credentials_available=false` |
| Network exfiltration | foundation performs no network access |
| Artifact execution | foundation has no execution API |
| Consumer mistakes completeness for safety | mandatory `consumer_verdict_required=true` |

## Future dynamic threats

- hypervisor or kernel escape;
- VM/sandbox fingerprinting and delayed execution;
- network pivoting and DNS rebinding;
- persistence across analyses;
- host filesystem access;
- packet-capture leakage;
- poisoned analyzer images or rules;
- evidence tampering and replay.

These require separate ADRs, attack-path validation, signed images, host-level egress control, and adversarial regression suites before dynamic profiles can return `completed`.
