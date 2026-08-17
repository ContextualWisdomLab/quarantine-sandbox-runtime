# Architecture

## Responsibility boundary

```text
Trigger adapter                     Consumer control plane
(email/upload/GitHub/API)           (Wardnet or equivalent)
        |                                      ^
        v                                      |
versioned request                              evidence bundle
        |                                      |
        +----> Quarantine Sandbox Runtime -----+
               - artifact identity
               - static analyzers
               - isolated worker adapters
               - evidence provenance
               - runtime attestation
```

The runtime is an execution and evidence plane. It is not a SIEM, SOAR, WAF, incident system, user directory, mailbox, or storage authority.

## Foundation components

### Contract boundary

Rejects malformed request metadata before artifact processing and keeps consumer-facing structures stable through schema versions.

### Ingestion boundary

Owns byte limits, immutable hash identity, non-executing format classification, and original-byte preservation in process memory.

### Static analyzer boundary

Accepts an immutable ingested artifact and returns typed findings. An analyzer cannot set the final runtime disposition or consumer verdict.

### Evidence normalizer

Assigns deterministic sequence numbers and IDs, records producer attribution, and preserves failures.

### Runtime manifest

Records runtime name, package version, source revision, requested profile, and security-boundary facts.

## Deployment evolution

- Foundation: library embedded in a trusted service with static-only behavior.
- Linux dynamic: scheduler plus disposable Firecracker or gVisor workers.
- Windows dynamic: separately managed Windows VM pool.
- Enterprise: queue, object storage, attestation signing, tenancy, quotas, observability, and disaster recovery.

Control-plane and worker credentials remain separate. An analysis worker receives no production credential and cannot directly mutate Wardnet incidents or source systems.
