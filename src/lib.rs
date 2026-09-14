#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Source-agnostic hostile-workload isolation and artifact-analysis runtime.
//!
//! The runtime owns reusable sandbox execution, isolation policy enforcement,
//! bounded service leases, artifact identity, and analysis evidence. Consumers
//! such as Wardnet and Chat/Agent control planes retain verdict, authorization,
//! conversation, task, tool-selection, secret, and user-action authority.

mod application_service;
mod artifact_analysis;
mod infrastructure;
mod pr_source_artifact;
mod sandbox_execution;

pub use application_service::{
    ApplicationServiceBackend, ApplicationServiceCoordinator, ApplicationServiceCoordinatorError,
    ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt, ExpiredLeaseCleanupResult,
    IsolationAttestation, LeaseOwnerId, ServiceEndpoint, ServiceProtocol,
};
pub use artifact_analysis::{
    AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest, AnalyzerFailure,
    AnalyzerFinding, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    FormatAnalyzer, IngestedArtifact, IngestionError, IngestionPolicy, RuntimeDisposition,
    RuntimeManifest, StaticAnalyzer, ingest_bytes, to_pretty_json,
};
pub use infrastructure::{PodmanLaunchPlan, RootlessPodmanAdapter};
#[cfg(unix)]
pub use infrastructure::{
    RuntimeGateArtifact, RuntimeGateArtifactError, RuntimeGateCommandBindingPlan,
    RuntimeGatePodmanAdapter,
};
pub use pr_source_artifact::{
    PrSourceArtifactError, PrSourceArtifactInput, PrSourceArtifactReceipt, StagedPrSourceArtifact,
    stage_pr_source_artifact,
};
pub use sandbox_execution::{
    BackendInvocationFailureKind, CommandExecutionBackend, CommandExecutionError,
    CommandExecutionRequest, CommandExecutionResult, IsolationControlStatus, IsolationPolicy,
    ResourceRequest, SandboxExecutionError, SandboxRuntimeError, VerifiedIsolationState,
    execute_command,
};
/// Compatibility name retained for the application-service public error contract.
pub use sandbox_execution::SandboxRuntimeError as ApplicationServiceError;
