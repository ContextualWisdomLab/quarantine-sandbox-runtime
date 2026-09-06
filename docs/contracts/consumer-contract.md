# Consumer Contract

This document defines the consumer-neutral boundary for Quarantine Sandbox Runtime. Consumer repositories keep their own domain authority and use an Anti-Corruption Layer to call a released runtime artifact.

## Version surface

The first release candidate carries independently versioned wire contracts rather than one implicit repository version:

| Contract | Current schema |
| --- | --- |
| Artifact analysis request | `1.0.0` |
| Artifact analysis evidence | `1.1.0` |
| Application-service request | `1.0.0` |
| Isolation policy | `1.0.0` |
| Application-service lease | `1.2.0` |
| Application-service cleanup receipt | `1.0.0` |
| Command-execution request | `1.0.0` |
| Command-execution result | `1.0.0` |
| Release evidence manifest | `1.0.0` |

All public JSON Schemas use Draft 2020-12 and reject unknown top-level fields. A consumer therefore validates an exact schema version it explicitly supports; it must not infer compatibility from the repository package version or silently coerce an unknown contract revision. A breaking field removal, meaning change, or relaxation requires a new major contract version. A minor contract revision may add security evidence that is required for that revision, as the application-service lease `1.2.0` did for backend version and effective seccomp/LSM/resource-control state and artifact-analysis evidence `1.1.0` does for job-identity binding evidence. Strict consumers upgrade deliberately rather than treating those additions as wire-compatible with earlier revisions.

The runtime may preserve multiple schema readers/writers in a future transport, but version negotiation must be explicit and bounded. Unsupported versions fail closed with stable transport-level error codes when a transport exists.

## Artifact analysis

A security or ingestion consumer submits:

- artifact bytes it already possesses;
- versioned `AnalysisRequest` schema `1.0.0`;
- optional closed `bounded_source_context` only for non-secret correlation metadata.

The runtime returns `EvidenceBundle` schema `1.1.0`. The v1.1 evidence revision preserves `analysis_job_id` as the opaque deterministic correlation identifier used by v1.0 and adds required `analysis_job_identity_sha256`. That companion SHA-256 binds the published job ID to receipt-visible request ID, requested profile, artifact SHA-256, one unambiguous runtime policy ID, and runtime source revision. JSON Schema validates the digest shape; the Rust validator recomputes cross-field equality. The archived v1.0 evidence schema remains available as `schemas/evidence-bundle-1.0.0.schema.json` for compatibility inspection, while the current and versioned v1.1 schemas are `schemas/evidence-bundle.schema.json` and `schemas/evidence-bundle-1.1.0.schema.json`.

The companion digest is self-consistency evidence, not producer authentication. A party able to rewrite the entire unsigned receipt can recompute it; consumers must not treat it as a substitute for signed provenance, immutable worker identity, or stable analyzer attestation. `RuntimeDisposition` describes analysis completeness only. `consumer_verdict_required` remains true. The consumer decides maliciousness, incident state, quarantine/block/allow/review, notification, and retention.

Artifact-analysis output is risk evidence. A hash, static finding, dynamic trace, or analyzer score is not authoritative truth about the binary/application and must not be projected as an authoritative Enterprise Architecture fact.

## Isolated application service

An Agent/Chat/tool consumer submits an `ApplicationServiceRequest` only after the consumer has already authorized the application for the relevant tenant/session/task.

### Request fields

```text
schema_version
request_id
image_reference      registry/storage image name + immutable @sha256 digest only
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

The image reference must be a normal container registry/storage name pinned by a lower-case SHA-256 digest. Explicit Podman/container-image transports such as `dir:`, `docker-archive:`, `docker-daemon:`, `oci:`, `oci-archive:`, and `containers-storage:` are rejected because they can redirect resolution to host-backed paths, alternate local stores, or another runtime authority. A registry authority may include a numeric port, for example `localhost:5000/cwl/tool@sha256:…`.

The operator supplies `IsolationPolicy`; the consumer cannot raise policy maxima.

### Returned lease

`ApplicationServiceLease` schema `1.2.0` provides:

```text
schema_version
request_id
image_reference
backend_id
backend_version
sandbox_id
network_id
policy_id
policy_sha256       canonical identity of every effective policy field
endpoint
  host               127.0.0.1
  port               random host port
  protocol
started_at_epoch_seconds
expires_at_epoch_seconds
shutdown_grace_seconds
isolation_attestation
  rootless
  read_only_root_filesystem
  all_capabilities_dropped
  no_new_privileges
  isolated_user_namespace
  external_egress_denied
  loopback_only_publication
  seccomp_enforced
  lsm_enforced
  resource_limits_verified
  credentials_available
```

The control fields distinguish positively verified, unavailable, and not-applicable states where the contract permits those states. The P0 application-service path returns a successful lease only after every required effective control is positively verified. In particular, capability-drop evidence covers both effective and bounding capability sets; configured command-line intent alone is insufficient.

A lease means the runtime reached its isolation/readiness contract. It does **not** mean the consumer was authorized to request it; authorization is the consumer's responsibility and must happen before launch.

### Cleanup

Every consumer terminal path—success, cancellation, timeout, workflow failure, or user abort—must request lease termination. `CleanupReceipt` proves container/network removal only when both removal operations succeed. Consumers must surface cleanup failure rather than treating it as successful termination.

The runtime also applies a Podman container timeout matching the requested lease. A durable orphan/network reaper remains required before a distributed restart-recovery claim.

## Bounded command execution

A CI-style consumer, such as a central review pipeline that must execute a pull request's own test suite or a proof-of-concept command and report structured evidence, submits a `CommandExecutionRequest` instead of an `ApplicationServiceRequest`:

```text
schema_version
request_id
image_reference      registry/storage image name + immutable @sha256 digest only
command               bounded direct argv; no shell; at least one entry required
resources
  memory_bytes
  cpu_millicores
  maximum_processes
  lease_seconds
  tmpfs_bytes
```

The command contract reuses the same registry-only image identity and resource policy as the application-service path. Explicit image transports resolving host paths or alternate stores are rejected before any backend invocation.

`execute_command` validates the request against the operator's `IsolationPolicy` and delegates to a `CommandExecutionBackend`, which runs the command to completion without a readiness-gated endpoint. It returns a `CommandExecutionResult`:

```text
schema_version
request_id
image_reference
backend_id
backend_version
sandbox_id
exit_code
timed_out
stdout
stdout_truncated
stderr
stderr_truncated
started_at_epoch_seconds
finished_at_epoch_seconds
```

`exit_code` is the sandboxed workload's process exit status. A nonzero value is observed workload evidence, not a runtime error; the consumer decides how that evidence affects its own verdict. `CommandExecutionError` is reserved for request validation or runtime/backend failure, including inability to establish or verify the sandbox.

`RootlessPodmanAdapter::run_command_at` is the production `CommandExecutionBackend`. Each invocation creates a fresh digest-pinned, read-only-rootfs, non-root sandbox with `--network none`, dropped capabilities, no-new-privileges, isolated namespaces, and bounded CPU/RAM/PID/tmpfs/lifetime controls. The adapter starts the container detached and requires live `podman top` evidence for effective seccomp, LSM, and capability state before accepting the sandbox. If a short-lived workload exits before live process evidence can be sampled, execution fails closed; static `container inspect` configuration is not a substitute. Supporting such workloads requires a reviewed start/hold/attest/release handshake or an equivalently stronger backend primitive.

An external caller reaches the backend through `quarantine-sandbox-runtime run --image <digest> [resource flags] -- <command> [args...]`. The CLI emits `CommandExecutionResult` JSON on stdout for a completed sandboxed workload and preserves the workload's own exit status; request/policy/backend failures are typed runtime failures rather than synthetic workload results. The command contract has no network-policy field and does not permit Internet egress. There is no released package or HTTP transport yet; consumers may depend only on a future immutable publication, not this branch.

## Forbidden consumer coupling

Consumers must not:

- import or copy internal Podman/gVisor/containerd modules;
- shell out directly to Podman/containerd as a substitute for this runtime contract;
- pass mutable image tags or explicit image transports that resolve host paths/alternate stores;
- pass prompt text, message bodies, credentials, arbitrary environment variables, broad host paths, runtime sockets, or host devices through the application-service or command-execution request;
- treat runtime evidence as verdict policy;
- expose the returned service endpoint beyond the reviewed loopback/co-location topology;
- reuse an expired lease;
- read runtime persistence through cross-service SQL;
- treat a pull-request head, branch artifact, or queued workflow as a production dependency.

## Controlled egress and secrets

P0 has neither. If an application needs them, the required capability must be implemented by a later versioned profile with explicit consumer authorization and an accepted security ADR. Enabling arbitrary Internet access or ambient secret environment variables is not a compatible P0 extension.

## Immutable consumption

Review and stacked development may refer to an exact full source commit SHA as provisional evidence. Production integration requires a published release artifact plus its immutable SHA-256 and verified provenance/SBOM attestations. A branch, pull-request head, tag, semantic version label, or transient Actions artifact alone is not sufficient immutable consumer identity.

After a release, Wardnet and contextual-orchestrator owner paths record the released package version, source SHA, package SHA-256, exact contract schema versions, and attestation verification. Their ACLs must reject a response that does not match the pinned contract/runtime identity. Consumers retain their own verdict, authorization, orchestration, secret, and user-action authority throughout upgrades and rollback.
