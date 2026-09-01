#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Source-agnostic hostile-workload isolation and artifact-analysis runtime.
//!
//! The runtime owns reusable sandbox execution, isolation policy enforcement,
//! bounded service leases, artifact identity, and analysis evidence. Consumers
//! such as Wardnet and Chat/Agent control planes retain verdict, authorization,
//! conversation, task, tool-selection, secret, and user-action authority.

mod application_service;
mod contracts;
mod ingestion;
mod runtime;
mod sandbox_execution;

pub use application_service::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
    IsolationAttestation, ServiceEndpoint, ServiceProtocol,
};
pub use contracts::{
    AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    RuntimeDisposition, RuntimeManifest,
};
pub use ingestion::{IngestedArtifact, IngestionError, IngestionPolicy, ingest_bytes};
pub use runtime::{
    AnalysisEngine, AnalysisError, AnalyzerFailure, AnalyzerFinding, FormatAnalyzer,
    StaticAnalyzer, to_pretty_json,
};
pub use sandbox_execution::{
    IsolationPolicy, PodmanLaunchPlan, ResourceRequest, RootlessPodmanAdapter,
};
