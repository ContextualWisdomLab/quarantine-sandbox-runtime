# ADR 0001: Product and authority boundary

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

The same isolation primitive is needed by more than one ContextualWisdomLab product. Wardnet needs hostile-artifact analysis and future detonation. Chat/Agent control planes need to start an explicitly approved application as a temporary service without granting it host or credential authority. Putting sandbox lifecycle inside each consumer would duplicate a privileged boundary and couple domain code to Podman/containerd/gVisor details.

## Decision

This repository owns reusable sandbox execution, isolation-policy enforcement, resource limits, service leases/endpoints, cleanup evidence, runtime attestation, and artifact-analysis execution/evidence.

It does **not** own:

- Wardnet maliciousness verdict, incident, quarantine/block, notification, or retention policy;
- `contextual-orchestrator` conversation, agent, task, tool, application-selection, authorization, secret, or user-action policy;
- naruon mailbox/admission truth;
- EgressWeave outbound HTTP authority;
- identity, billing, or unrelated product state.

Consumer-specific objects are translated through versioned Anti-Corruption Layers. The runtime neither reads foreign application databases nor vendors sibling source.

## Consequences

- Wardnet and Chat/Agent systems can reuse one hardened isolation boundary without sharing business authority.
- A backend implementation can change without forcing consumer domain models to change.
- Runtime evidence is not a maliciousness verdict and a service lease is not proof of consumer authorization.
- Any future repository split must be justified by stable deployment/reuse responsibility, not by technical-layer aesthetics.
