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
mod sandbox_execution;

pub use application_service::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationAttestation, ServiceEndpoint, ServiceProtocol,
};
pub use artifact_analysis::{
    AnalysisEngine, AnalysisError, AnalysisProfile, AnalysisRequest, AnalyzerFailure,
    AnalyzerFinding, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    FormatAnalyzer, IngestedArtifact, IngestionError, IngestionPolicy, RuntimeDisposition,
    RuntimeManifest, StaticAnalyzer, ingest_bytes, to_pretty_json,
};
pub use sandbox_execution::{
    IsolationPolicy, PodmanLaunchPlan, ResourceRequest, RootlessPodmanAdapter,
};
