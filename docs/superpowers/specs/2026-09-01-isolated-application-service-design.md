# Isolated Application Service Design

## Status

Accepted for implementation by the user request dated 2026-09-01.

## Problem

Quarantine Sandbox Runtime began as a source-agnostic artifact-analysis runtime. That is too narrow for the shared isolation responsibility now required across ContextualWisdomLab. Wardnet needs hostile-artifact analysis/detonation, while Chat/Agent control planes such as `contextual-orchestrator` also need to start a specifically approved application as a short-lived service without granting that application ambient host authority.

The isolation primitive is the reusable product responsibility. Malware verdicts, chat/agent orchestration, tool choice, conversation state, and consumer secrets remain external authorities.

## Approaches considered

### 1. Put container isolation inside Wardnet and contextual-orchestrator independently

Rejected. It duplicates a privileged security boundary, produces divergent policies, and makes each consumer responsible for Podman/gVisor/containerd lifecycle details.

### 2. Create another generic container repository

Rejected for now. `quarantine-sandbox-runtime` already owns isolation-oriented runtime execution and evidence. Creating another repository before the boundary proves independent release/deployment pressure would split one cohesive responsibility.

### 3. Broaden this repository into reusable sandbox execution with consumer-specific supporting contexts

Accepted. The repository keeps its name but its internal architecture becomes a modular sandbox runtime with a Core `sandbox_execution` bounded context and separate supporting contexts for `artifact_analysis` and `application_service`.

## Domain model and Context Map

### Core: `sandbox_execution`

Owns:

- immutable workload identity inputs;
- isolation policy and resource budget validation;
- sandbox lifecycle and lease identity;
- backend execution port;
- service endpoint receipt;
- termination and cleanup receipts;
- runtime isolation attestation.

It does not know Wardnet incidents or Chat/Agent conversations.

### Supporting: `artifact_analysis`

Owns:

- immutable artifact byte identity;
- bounded source context;
- static/dynamic analysis profiles;
- analyzer evidence;
- analysis completeness.

It may consume `sandbox_execution` for future detonation. Maliciousness verdicts remain consumer-owned.

### Supporting: `application_service`

Owns:

- approved application image/service request;
- container service port and protocol;
- bounded non-shell command arguments;
- conversion from application intent to sandbox workload;
- service lease returned to the consumer.

It does not decide which tool/application an Agent may use. That decision remains in the Chat/Agent control plane.

### Infrastructure adapters

The first adapter is rootless Podman. Future adapters may include containerd/gVisor `runsc` and Kubernetes RuntimeClass. Adapter-specific command syntax and runtime metadata must never become domain entities.

### External consumers

- Wardnet: verdict, incident, quarantine/block, notification, retention.
- contextual-orchestrator: chat, agent/task/tool policy, service selection, authorization, secret authority, user-visible action.
- EgressWeave or another approved egress owner: any future controlled outbound HTTP policy.

## P0 application-service contract

An `ApplicationServiceRequest` contains:

- schema version;
- consumer request identifier;
- immutable OCI image reference ending in `@sha256:<64 lowercase hex>`;
- one TCP container port;
- service protocol (`tcp` or `http`);
- zero or more bounded command arguments passed directly without a shell;
- requested CPU, memory, PID, lease, and tmpfs resources.

An operator-owned `IsolationPolicy` supplies maximum resource values, readiness timeout/poll interval, shutdown grace, numeric non-root UID/GID, and a stable policy identifier.

The request is rejected when it exceeds policy. There is no request flag for privileged mode, host networking, devices, writable root filesystem, extra capabilities, arbitrary mounts, or ambient environment variables.

## Rootless Podman P0 adapter

The adapter must:

1. require Podman to report rootless mode;
2. require the image to be already present and use `--pull=never`;
3. create a unique `--internal --disable-dns` bridge network for the sandbox;
4. create the container with read-only rootfs, all capabilities dropped, `no-new-privileges`, `--userns=auto`, an operator-policy non-root UID/GID, CPU/RAM/PID limits, and bounded tmpfs;
5. publish only `127.0.0.1::<container-port>/tcp`;
6. never mount Docker/Podman sockets or host devices;
7. never pass consumer/provider credentials or host environment through to the container;
8. start the container and resolve the random host loopback port;
9. require bounded readiness before returning the endpoint;
10. return an attested lease with immutable image identity, backend identity, sandbox/network IDs, resource-policy ID, start/expiry timestamps, and isolation facts;
11. provide bounded stop/remove/network-cleanup behavior.

A tag-only image is never accepted. Registry pulls are excluded from this profile so runtime startup does not become an uncontrolled egress path.

## Networking

P0 uses a per-sandbox Podman bridge created with `--internal` and DNS disabled. The application can receive loopback-published traffic from its consumer but does not receive an external route through that network. Arbitrary Internet egress is not a request option.

A later controlled-egress profile must be a distinct contract routed through an explicit approved proxy or egress owner. It may not silently turn the P0 profile into an Internet-connected container.

## Secrets

P0 is credential-free inside the application container. The consumer may keep credentials and call privileged providers itself, or expose a purpose-limited local capability proxy in a later accepted design. Direct secret environment variables, mounted credential directories, runtime sockets, SSH agents, cloud metadata access, and host keychains are outside P0.

## Failure and cleanup

Fail closed on:

- non-rootless backend;
- mutable image identity;
- invalid/bounded-field violations;
- resource-policy violation;
- network/container creation failure;
- start failure;
- malformed/non-loopback port mapping;
- readiness timeout;
- cleanup failures that prevent proving isolation resources were removed.

A readiness failure triggers cleanup before returning the error. A returned lease is not evidence of maliciousness or consumer authorization; it is evidence that an application service passed this runtime's isolation/startup contract.

## Testing

The first TDD increment proves:

- tag-only image rejection;
- resource cap enforcement;
- deterministic rootless Podman command plan;
- internal network and loopback-only publish;
- required read-only/capability/no-new-privilege/user-namespace controls;
- no privileged/host-network/runtime-socket option in the plan.

The next increment executes the real process boundary with a controlled fake Podman executable, then adds a real rootless Podman CI lane before making production-isolation claims. Command-plan tests are not container-isolation evidence by themselves.

## Standards and primary technical sources

- Podman rootless mode: user namespaces and non-root execution.
- Podman network `--internal`: external access restriction for bridge networks.
- Podman port publication: explicit loopback host binding.
- gVisor: OCI `runsc` application-kernel isolation for a stronger future backend.
- Open Containers Runtime Specification: backend-neutral OCI runtime semantics.

Detailed APA 7 references and decision-to-test traceability belong in `docs/doctoring/` with implementation changes.
