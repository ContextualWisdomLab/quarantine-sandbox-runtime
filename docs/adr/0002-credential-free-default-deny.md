# ADR 0002: Credential-free and default-deny standard profiles

- **Status:** Accepted
- **Date:** 2026-09-01

## Context

Host credentials, runtime-engine sockets, unrestricted network access, and broad host mounts turn an isolated workload into an ambient-authority extension of the caller. That is unacceptable for both hostile artifact detonation and Agent-started applications.

## Decision

The standard profiles are credential-free and default-deny:

- no consumer/provider secret values in application or artifact-worker environment variables;
- no Docker, Podman, containerd, SSH-agent, cloud-metadata, or host-keychain authority inside a sandbox;
- no privileged mode, host network, host PID/IPC, broad host filesystems, or host devices;
- no arbitrary external route in the P0 application-service network;
- only explicitly typed, bounded inputs are admitted;
- only loopback-scoped service publication is permitted by the P0 application-service profile.

A future secret-broker or controlled-egress profile must be a new reviewed contract with task-scoped authority, explicit consumer authorization, revocation/destruction semantics, and separate security tests. It may not silently widen the standard profile.

## Consequences

Some applications cannot run under the P0 profile. That is an intentional fail-closed result, not a reason to inject ambient credentials or host authority. Consumers may keep privileged provider calls outside the isolated application and expose a narrower capability in a future accepted design.
