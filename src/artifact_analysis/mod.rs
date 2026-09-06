//! Supporting bounded context for immutable artifact ingestion and analysis evidence.
//!
//! This context owns hostile artifact identity, static/dynamic analysis requests,
//! analyzer evidence, and analysis completeness. It does not own maliciousness
//! verdicts or incident-response policy. Future detonation consumes the Core
//! `sandbox_execution` context through an explicit port rather than embedding
//! container-backend details here.

mod analysis_engine;
mod claude_plugin_package;
mod contracts;
mod evidence_bundle;
mod ingestion;
mod runtime;

pub use analysis_engine::AnalysisEngine;
pub use claude_plugin_package::{
    CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE, ClaudePluginPackageAnalysisRequest,
    ClaudePluginPackageContractError,
};
pub use contracts::{
    AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind, BoundedSourceContext,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceKind, EvidenceRecord, RuntimeDisposition,
    RuntimeManifest,
};
pub use evidence_bundle::{EVIDENCE_BUNDLE_SCHEMA_VERSION, EvidenceBundle, to_pretty_json};
pub use ingestion::{IngestedArtifact, IngestionError, IngestionPolicy, ingest_bytes};
pub use runtime::{
    AnalysisError, AnalyzerFailure, AnalyzerFinding, FormatAnalyzer, StaticAnalyzer,
};
