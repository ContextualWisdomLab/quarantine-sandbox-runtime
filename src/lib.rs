#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Source-agnostic, credential-free artifact analysis runtime.
//!
//! The runtime establishes immutable artifact identity, invokes ordered
//! non-executing analyzers, and emits deterministic evidence. A consumer such
//! as Wardnet remains responsible for maliciousness verdicts and response
//! policy.

mod contracts;
mod ingestion;
mod runtime;

pub use contracts::{
    AnalysisContext, AnalysisProfile, AnalysisRequest, ArtifactDescriptor, ArtifactKind,
    CONTRACT_SCHEMA_VERSION, ContractError, EvidenceBundle, EvidenceKind, EvidenceRecord,
    RuntimeDisposition, RuntimeManifest,
};
pub use ingestion::{IngestedArtifact, IngestionError, IngestionPolicy, ingest_bytes};
pub use runtime::{
    AnalysisEngine, AnalysisError, AnalyzerFailure, AnalyzerFinding, FormatAnalyzer,
    StaticAnalyzer, to_pretty_json,
};
