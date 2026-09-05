# Architecture Decision Records

| ADR | Status | Decision |
| --- | --- | --- |
| [0001](0001-product-authority-boundary.md) | Accepted | Reusable isolation/evidence runtime; consumer products retain business and response authority. |
| [0002](0002-credential-free-default-deny.md) | Accepted | Standard sandbox profiles are credential-free and default-deny network/host authority. |
| [0003](0003-published-contract-consumption.md) | Accepted | Consumers integrate through immutable published contracts/artifacts and ACLs, not sibling source or backend SDKs. |
| [0004](0004-truthful-capability-claims.md) | Accepted | Planned analyzers/backends/integrations are not represented as implemented or release-ready. |
| [0005](0005-sandbox-execution-context.md) | Accepted | `sandbox_execution` is the Core bounded context; `artifact_analysis` and `application_service` are Supporting contexts. |
| [0006](0006-isolated-application-service.md) | Proposed | Rootless Podman is the proposed first isolated application-service adapter; acceptance requires protected integration plus current-head real-runtime evidence. |
| [0007](0007-bounded-command-execution-contract.md) | Proposed | A bounded run-to-completion command contract is proposed alongside service leases; acceptance requires protected integration and fresh contract/security evidence. |
| [0008](0008-podman-backed-command-execution-and-cli.md) | Proposed | A rootless-Podman command backend and synchronous CLI are proposed; static-only attestation is rejected and acceptance requires effective-runtime/cleanup/release evidence. |

An ADR whose acceptance depends on implementation or runtime evidence stays Proposed while that evidence exists only on an unmerged/Draft candidate. Promote it to Accepted only after the decision is integrated into the protected branch and the required current-head evidence is revalidated under live governance.

Superseding an Accepted ADR requires a new ADR that names the superseded decision. Changing implementation detail without changing a binding architectural decision does not require a new ADR, but the relevant ADR and traceability must remain code-current.
