//! Supporting bounded context for immutable artifact ingestion and analysis evidence.
//!
//! This context owns hostile artifact identity, static/dynamic analysis requests,
//! analyzer evidence, and analysis completeness. It does not own maliciousness
//! verdicts or incident-response policy. Future detonation consumes the Core
//! `sandbox_execution` context through an explicit port rather than embedding
//! container-backend details here.

mod analyzer_worker;
mod claude_plugin_package;
mod contracts;
mod ingestion;
mod runtime;

pub use analyzer_worker::{
    AnalyzerWorkerBudget, AnalyzerWorkerContractError, AnalyzerWorkerExecutionError,
    AnalyzerWorkerExecutionPort, AnalyzerWorkerFinding, AnalyzerWorkerIdentity,
    AnalyzerWorkerIsolationEvidence, AnalyzerWorkerOutcome, AnalyzerWorkerReceipt,
    AnalyzerWorkerRequest,
};
pub use claude_plugin_package::{
    CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE, ClaudePluginPackageAnalysisRequest,
    ClaudePluginPackageContractError,
};
pub use contracts::{
    AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    RuntimeDisposition, RuntimeManifest,
};
pub use ingestion::{IngestedArtifact, IngestionError, IngestionPolicy, ingest_bytes};
pub use runtime::{
    AnalysisEngine, AnalysisError, AnalyzerFailure, AnalyzerFinding, FormatAnalyzer, StaticAnalyzer,
    to_pretty_json,
};
