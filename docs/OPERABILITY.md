# Operability and Recovery

## Operating model

The current crate is an embeddable runtime library. It owns sandbox lifecycle and evidence semantics, not a public multi-tenant control-plane API. A future service wrapper must preserve the same domain boundary and add authentication, tenancy, queueing, durable lease state, and observability through explicit adapters.

## Host prerequisites for rootless Podman profile

- Linux host with a supported rootless Podman configuration;
- subordinate UID/GID mappings and rootless user namespace support as required by Podman;
- a cgroup configuration that can enforce requested CPU/RAM/PID limits;
- local availability of the immutable application image because launch uses `--pull=never`;
- loopback host port availability;
- sufficient disk/tmpfs capacity within operator policy;
- no expectation that the application can access host devices, runtime sockets, external Internet, or consumer secrets.

If a required host capability cannot be proven, the profile is unavailable rather than silently weakened.

## Launch lifecycle

```text
validate request/policy
→ verify rootless backend
→ create internal network
→ create bounded container
→ start
→ resolve loopback port
→ bounded readiness
→ return lease
```

Every intermediate failure attempts removal of resources created before the failure. A cleanup failure replaces a superficially successful/ordinary error result because the operator must know isolation resources may remain.

## Termination lifecycle

```text
consumer terminal state
→ stop with recorded grace
→ force-remove container
→ remove per-sandbox network
→ cleanup receipt
```

Consumers must call termination for success, cancellation, timeout, orchestration failure, and user abort.

## Lease expiration

The Podman container timeout mirrors `lease_seconds`, which bounds application process lifetime even if the embedding process does not request termination. This is not sufficient orphan recovery: stopped containers and networks can remain after control-process failure.

### GA recovery requirement

Implement a durable lease/orphan reaper that:

- records active lease identity and expected expiry without storing consumer secrets;
- scans only resources carrying this runtime's labels/ownership identity;
- never deletes unrelated containers/networks;
- reclaims expired or terminal leases idempotently;
- records cleanup attempts/outcomes;
- runs on startup and periodically;
- has a bounded scan/work budget;
- can reconcile a missing container or missing network without treating it as a security success unless ownership/lifecycle is understood.

Durable persistence requires its own ADR and migration/recovery plan.

## Health and readiness for a future service wrapper

- **liveness:** runtime process event loop/control path alive;
- **startup:** backend prerequisites and policy loaded;
- **readiness:** new requests can be safely accepted under configured backend/resource capacity;
- **degraded:** one optional backend unavailable while another explicitly configured equivalent profile remains available;
- **not ready:** required isolation/backend/resource enforcement unavailable.

Do not return ready merely because the HTTP/process shell is alive.

## Retained limited-VM observation audit

`scripts/audit_runner_boundary_receipt.py` is an offline operator-support
adapter for the retained `sdp-vm-boundary-20261003` receipt profile. It is not
an application-service admission API or a replacement runner provisioner.
The historical profile fixes its two systemd unit names and QEMU executable;
new producer profiles require a separately reviewed contract.

Run from the repository root with independently retained input files:

```sh
python3 scripts/audit_runner_boundary_receipt.py \
  --summary /approved/local/summary.json \
  --control-serial /approved/local/control.log \
  --denied-serial /approved/local/denied.log \
  --expected-summary-sha256 "$PINNED_SUMMARY_SHA256"
python3 -m unittest scripts.test_audit_runner_boundary_receipt
```

The caller must pin the expected summary digest through a separate custody
path. Computing it from the supplied summary establishes only content
identity, not operator authentication. The tool joins that digest, both
serial digests, one complete guest result per serial, the shared transaction,
separate process identities, cgroups, observed attachment IDs, point results,
fixture hits, and producer-declared cleanup. It neither executes artifact
bytes nor contacts a host, starts a VM, registers a runner, or changes policy.

Inputs must be regular files with no final-component symlink. Reads are
bounded to 256 KiB for the summary and 1 MiB for each serial. Duplicate JSON
keys, nonfinite/floating numbers, invalid UTF-8, excessive decoded nesting,
missing fields, type confusion, inconsistent observations, failed cleanup,
and promoted full-boundary/clean-job/registered-CI flags are rejected. Exit
`1` emits a fixed `receipt_rejected` diagnostic without raw input or paths.
Exit `0` means only that this retained, limited observation is internally
consistent. Both exits preserve `lease_allowed=false` and
`operator_authenticated=false`.

An attachment ID is not verified BPF program semantics. A timeout at two
private TCP endpoints is not whole-CIDR denial. Producer-declared fsync,
pristine paths, or unit cleanup is not a newly observed runtime operation.
The audit does not establish current enforcement, service endpoint identity,
source/image provenance, operator approval, actual clean-per-job execution,
or exact-head native CI acceptance. Keep untrusted/public CI leasing disabled
until the original pre-lease and in-job isolation gates are independently met.

## Observability

Low-cardinality metrics should include:

- launch request/result count by profile/backend/error code;
- launch-to-ready latency;
- readiness timeout count;
- explicit termination count;
- cleanup failure count;
- active lease count;
- expired/orphan lease count;
- backend prerequisite failures;
- requested/authorized resource totals by bounded buckets;
- analysis request/analyzer failure counts;
- evidence bundle disposition counts.

Logs/traces must not include artifact bytes, credentials, raw prompts/messages, or unbounded command/output text. Use opaque request/sandbox IDs and stable error codes.

## Capacity and backpressure

Before a network service is GA, add operator-controlled global/per-tenant concurrency and resource admission. Do not create a sandbox if total reserved CPU/RAM/PID/tmpfs capacity would exceed host policy. Long-running requests must be rejected or queued durably rather than creating unbounded local processes.

## Incident response

### Suspected sandbox escape

1. Stop accepting new work on affected backend/profile.
2. Preserve host/runtime/audit evidence without running suspect workload tools on the host.
3. Rotate any host-level credential that may have been exposed even though P0 workloads should receive none.
4. Isolate/rebuild affected host from known-good image if integrity cannot be established.
5. Correlate immutable application/artifact digest, runtime version, backend version, policy, sandbox ID, timestamps, and consumer request reference.
6. Patch at the owning boundary and add a regression/real isolation test before restoring service.

### Cleanup/orphan incident

1. Stop new launches if resource ownership cannot be safely determined.
2. Enumerate only runtime-labeled containers/networks.
3. Reconcile active lease records when durable state exists.
4. Remove expired owned resources idempotently.
5. Record every remediation and verify no unrelated container/network was touched.

### Resource-exhaustion incident

- reject new work before weakening limits;
- inspect reserved vs actual resource use;
- reclaim expired/orphan resources;
- keep user-facing service unavailable/degraded rather than launching without enforceable bounds.

## Backup/restore

Current in-process implementation has no durable database and therefore no database backup claim. When durable lease/evidence persistence is added, backup/restore rehearsal must verify:

- schema compatibility;
- active/expired lease reconciliation;
- evidence hash integrity;
- encryption key availability where encryption is introduced;
- idempotent orphan cleanup after restore;
- no consumer secret restoration into workload context.

## Rollback

A release rollback is permitted only if the prior runtime version supports every active serialized contract or all active leases/jobs are drained first. Backend/image policy changes require compatibility analysis; never reinterpret a new isolation attestation as if an older runtime had enforced it.

## Service-level objectives

No production SLO is claimed yet. Establish SLOs only after real backend load/soak measurements. Initial acceptance measurements should cover launch-to-ready, cleanup latency, successful cleanup ratio, resource enforcement, orphan count, and availability under bounded concurrent load.
