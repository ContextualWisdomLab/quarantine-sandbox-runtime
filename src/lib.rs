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
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    CommandExecutionBackend, CommandExecutionError, CommandExecutionRequest,
    CommandExecutionResult, ExpiredLeaseCleanupResult, IsolationAttestation, LeaseOwnerId,
    ServiceEndpoint, ServiceProtocol, execute_command,
};
pub use artifact_analysis::{
    AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest, AnalyzerFailure,
    AnalyzerFinding, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE, CONTRACT_SCHEMA_VERSION,
    ClaudePluginPackageAnalysisRequest, ClaudePluginPackageContractError, ContractError,
    EvidenceBundle, EvidenceKind, EvidenceRecord, FormatAnalyzer, IngestedArtifact, IngestionError,
    IngestionPolicy, RuntimeDisposition, RuntimeManifest, StaticAnalyzer, ingest_bytes,
    to_pretty_json,
};
pub use infrastructure::{PodmanLaunchPlan, RootlessPodmanAdapter};
pub use pr_source_artifact::{
    PrSourceArtifactError, PrSourceArtifactInput, PrSourceArtifactReceipt, StagedPrSourceArtifact,
    stage_pr_source_artifact,
};
pub use sandbox_execution::{
    IsolationControlStatus, IsolationPolicy, ResourceRequest, SandboxExecutionError,
    VerifiedIsolationState,
};

impl From<SandboxExecutionError> for ApplicationServiceError {
    fn from(error: SandboxExecutionError) -> Self {
        match error {
            SandboxExecutionError::InvalidPolicy { field_name } => {
                Self::InvalidPolicy { field_name }
            }
            SandboxExecutionError::ResourceLimitExceeded { resource_name } => {
                Self::ResourceLimitExceeded { resource_name }
            }
        }
    }
}
