# Consumer Contract

This document defines the consumer-neutral boundary for Quarantine Sandbox Runtime. Consumer repositories keep their own domain authority and use an Anti-Corruption Layer to call this runtime.

## Artifact analysis

A security or ingestion consumer submits:

- artifact bytes it already possesses;
- versioned `AnalysisRequest`;
- optional closed `bounded_source_context` only for non-secret correlation metadata.

The runtime returns a versioned `EvidenceBundle`. `RuntimeDisposition` describes analysis completeness only. `consumer_verdict_required` remains true. The consumer decides maliciousness, incident state, quarantine/block/allow/review, notification, and retention.

## Isolated application service

An Agent/Chat/tool consumer submits an `ApplicationServiceRequest` only after the consumer has already authorized the application for the relevant tenant/session/task.

### Request fields

```text
schema_version
request_id
image_reference      immutable OCI @sha256 digest only
container_port
protocol             tcp | http
command              bounded direct argv; no shell
resources
  memory_bytes
  cpu_millicores
  maximum_processes
  lease_seconds
  tmpfs_bytes
```

The operator supplies `IsolationPolicy`; the consumer cannot raise policy maxima.

### Returned lease

`ApplicationServiceLease` provides:

```text
schema_version
request_id
image_reference
backend_id
sandbox_id
network_id
policy_id
endpoint
  host               127.0.0.1
  port               random host port
  protocol
started_at_epoch_seconds
expires_at_epoch_seconds
shutdown_grace_seconds
isolation_attestation
```

P0 attestation states rootless execution, read-only rootfs, all capabilities dropped, no-new-privileges, user-namespace isolation, denied external egress, loopback-only publication, and absence of consumer/provider credentials.

A lease means the runtime reached its isolation/readiness contract. It does **not** mean the consumer was authorized to request it; authorization is the consumer's responsibility and must happen before launch.

### Cleanup

Every consumer terminal path—success, cancellation, timeout, workflow failure, or user abort—must request lease termination. `CleanupReceipt` proves container/network removal only when both removal operations succeed. Consumers must surface cleanup failure rather than treating it as successful termination.

The runtime also applies a Podman container timeout matching the requested lease. A durable orphan/network reaper remains required before GA for process-crash recovery.

## Forbidden consumer coupling

Consumers must not:

- import or copy internal Podman/gVisor/containerd modules;
- shell out directly to Podman/containerd as a substitute for this runtime contract;
- pass mutable image tags;
- pass prompt text, message bodies, credentials, arbitrary environment variables, broad host paths, runtime sockets, or host devices through the application-service request;
- treat runtime evidence as verdict policy;
- expose the returned service endpoint beyond the host loopback boundary;
- reuse an expired lease.

## Controlled egress and secrets

P0 has neither. If an application needs them, the required capability must be implemented by a later versioned profile with explicit consumer authorization and an accepted security ADR. Enabling arbitrary Internet access or ambient secret environment variables is not a compatible P0 extension.

## Immutable consumption

During review a consumer may pin an exact full source commit SHA. Production integration requires a published package/container artifact pinned by immutable digest with verified provenance. A branch, tag, or semantic version label alone is not immutable identity.
