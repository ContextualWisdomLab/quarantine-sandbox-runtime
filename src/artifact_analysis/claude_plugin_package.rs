//! Credential-free Claude plugin package analysis request contract.
//!
//! This module owns admission-time validation only. It does not execute plugin
//! content, resolve marketplace state, authorize egress, or make an admission
//! verdict for Noema or any consumer.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Wire profile code for credential-free Claude plugin package analysis.
pub const CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE: &str = "claude_plugin_package_analysis";

const MAX_TEXT_BYTES: usize = 512;
const MAX_PATH_BYTES: usize = 1_024;
const MAX_PROBE_COUNT: usize = 32;
const MAX_CPU_MILLISECONDS: u64 = 300_000;
const MAX_MEMORY_BYTES: u64 = 2_147_483_648;
const MAX_PROCESS_COUNT: u64 = 256;
const MAX_DISK_BYTES: u64 = 8_589_934_592;
const MAX_OUTPUT_BYTES: u64 = 16_777_216;
const MAX_WALL_TIME_SECONDS: u64 = 3_600;

const ALLOWED_PROBE_CODES: &[&str] = &[
    "package_inventory",
    "manifest_consistency",
    "hook_discovery",
    "hook_dry_run",
    "command_discovery",
    "command_dry_run",
    "skill_agent_prompt_boundary",
    "mcp_stdio_startup",
    "mcp_stdio_schema_enumeration",
    "mcp_remote_connection_attempt",
    "filesystem_access_probe",
    "network_attempt_probe",
    "process_spawn_probe",
    "package_lifecycle_probe",
    "cleanup_recovery_probe",
    "output_channel_probe",
];

/// Immutable, bounded request for the Claude plugin package quarantine profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudePluginPackageAnalysisRequest {
    /// Request contract version.
    pub schema_version: String,
    /// Caller correlation/idempotency identifier.
    pub request_id: String,
    /// Exact profile code; no caller-defined execution profile is accepted.
    pub profile: String,
    /// Immutable marketplace catalog repository in `owner/name` form.
    pub catalog_repository: String,
    /// Exact 40-character lower-case catalog commit SHA-1.
    pub catalog_commit_sha: String,
    /// Exact 40-character lower-case marketplace blob SHA-1.
    pub marketplace_blob_sha: String,
    /// SHA-256 of the canonical marketplace entry used for admission correlation.
    pub marketplace_entry_sha256: String,
    /// Catalog plugin name.
    pub plugin_name: String,
    /// Catalog plugin version text.
    pub plugin_version: String,
    /// Immutable source repository in `owner/name` form.
    pub source_repository: String,
    /// Exact 40-character lower-case source commit SHA-1.
    pub source_commit_sha: String,
    /// Canonical repository-relative source path.
    pub source_path: String,
    /// SHA-256 of the exact materialized artifact bytes to analyze.
    pub artifact_sha256: String,
    /// Opaque reference to the matching AppGuardrail static-scan receipt.
    pub appguardrail_scan_receipt_reference: String,
    /// Versioned analysis-profile identity.
    pub analysis_profile_id: String,
    /// SHA-256 of the exact versioned analysis-profile contract/configuration.
    pub analysis_profile_sha256: String,
    /// Explicit fixed probe codes requested from the profile allowlist.
    pub requested_probe_codes: Vec<String>,
    /// Maximum CPU budget in milliseconds.
    pub maximum_cpu_milliseconds: u64,
    /// Maximum memory budget in bytes.
    pub maximum_memory_bytes: u64,
    /// Maximum process count.
    pub maximum_process_count: u64,
    /// Maximum ephemeral disk budget in bytes.
    pub maximum_disk_bytes: u64,
    /// Maximum trusted result-channel output in bytes.
    pub maximum_output_bytes: u64,
    /// Maximum wall-clock execution time in seconds.
    pub maximum_wall_time: u64,
    /// Versioned deny-by-default network policy reference.
    pub network_policy_reference: String,
    /// Versioned ephemeral-filesystem policy reference.
    pub filesystem_policy_reference: String,
}

impl ClaudePluginPackageAnalysisRequest {
    /// Validate immutable identity, probe allowlist, paths, and resource ceilings.
    ///
    /// # Errors
    ///
    /// Returns [`ClaudePluginPackageContractError`] when untrusted request data
    /// is mutable, malformed, unbounded, or outside this fixed profile contract.
    pub fn validate(&self) -> Result<(), ClaudePluginPackageContractError> {
        if self.schema_version != "1.0.0" {
            return Err(ClaudePluginPackageContractError::UnsupportedSchemaVersion);
        }
        if self.profile != CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE {
            return Err(ClaudePluginPackageContractError::UnsupportedProfile);
        }

        for (field_name, value) in [
            ("request_id", self.request_id.as_str()),
            ("catalog_repository", self.catalog_repository.as_str()),
            ("plugin_name", self.plugin_name.as_str()),
            ("plugin_version", self.plugin_version.as_str()),
            ("source_repository", self.source_repository.as_str()),
            (
                "appguardrail_scan_receipt_reference",
                self.appguardrail_scan_receipt_reference.as_str(),
            ),
            ("analysis_profile_id", self.analysis_profile_id.as_str()),
            (
                "network_policy_reference",
                self.network_policy_reference.as_str(),
            ),
            (
                "filesystem_policy_reference",
                self.filesystem_policy_reference.as_str(),
            ),
        ] {
            validate_text(field_name, value, MAX_TEXT_BYTES)?;
        }

        for (field_name, value) in [
            ("catalog_commit_sha", self.catalog_commit_sha.as_str()),
            ("marketplace_blob_sha", self.marketplace_blob_sha.as_str()),
            ("source_commit_sha", self.source_commit_sha.as_str()),
        ] {
            if !is_lowercase_hex(value, 40) {
                return Err(ClaudePluginPackageContractError::InvalidGitSha { field_name });
            }
        }

        for (field_name, value) in [
            (
                "marketplace_entry_sha256",
                self.marketplace_entry_sha256.as_str(),
            ),
            ("artifact_sha256", self.artifact_sha256.as_str()),
            (
                "analysis_profile_sha256",
                self.analysis_profile_sha256.as_str(),
            ),
        ] {
            if !is_lowercase_hex(value, 64) {
                return Err(ClaudePluginPackageContractError::InvalidSha256 { field_name });
            }
        }

        validate_repository_relative_path(&self.source_path)?;
        validate_probe_codes(&self.requested_probe_codes)?;
        validate_resource(
            "maximum_cpu_milliseconds",
            self.maximum_cpu_milliseconds,
            MAX_CPU_MILLISECONDS,
        )?;
        validate_resource(
            "maximum_memory_bytes",
            self.maximum_memory_bytes,
            MAX_MEMORY_BYTES,
        )?;
        validate_resource(
            "maximum_process_count",
            self.maximum_process_count,
            MAX_PROCESS_COUNT,
        )?;
        validate_resource(
            "maximum_disk_bytes",
            self.maximum_disk_bytes,
            MAX_DISK_BYTES,
        )?;
        validate_resource(
            "maximum_output_bytes",
            self.maximum_output_bytes,
            MAX_OUTPUT_BYTES,
        )?;
        validate_resource(
            "maximum_wall_time",
            self.maximum_wall_time,
            MAX_WALL_TIME_SECONDS,
        )?;
        Ok(())
    }
}

/// Admission-time validation error for the package-analysis request contract.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ClaudePluginPackageContractError {
    /// The request schema revision is unsupported.
    #[error("unsupported Claude plugin package analysis schema version")]
    UnsupportedSchemaVersion,
    /// The request selected a profile other than the fixed package-analysis profile.
    #[error("unsupported Claude plugin package analysis profile")]
    UnsupportedProfile,
    /// A required text field is empty, oversized, or contains control text.
    #[error("invalid bounded text field: {field_name}")]
    InvalidText {
        /// Stable field name.
        field_name: &'static str,
    },
    /// A Git object identity is not canonical lower-case 40-character hexadecimal.
    #[error("invalid immutable Git SHA field: {field_name}")]
    InvalidGitSha {
        /// Stable field name.
        field_name: &'static str,
    },
    /// A SHA-256 identity is not canonical lower-case 64-character hexadecimal.
    #[error("invalid SHA-256 field: {field_name}")]
    InvalidSha256 {
        /// Stable field name.
        field_name: &'static str,
    },
    /// The source path is not canonical repository-relative form.
    #[error("source_path must be canonical and repository-relative")]
    InvalidSourcePath,
    /// The request contains no probes, too many probes, duplicates, or an unknown probe.
    #[error("requested_probe_codes violates the fixed probe allowlist")]
    InvalidProbeSet,
    /// A resource budget is zero or above the fixed profile ceiling.
    #[error("resource budget outside profile ceiling: {field_name}")]
    InvalidResourceBudget {
        /// Stable field name.
        field_name: &'static str,
    },
}

fn validate_text(
    field_name: &'static str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), ClaudePluginPackageContractError> {
    if value.is_empty() || value.len() > maximum_bytes || value.chars().any(char::is_control) {
        return Err(ClaudePluginPackageContractError::InvalidText { field_name });
    }
    Ok(())
}

fn is_lowercase_hex(value: &str, exact_len: usize) -> bool {
    value.len() == exact_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_repository_relative_path(value: &str) -> Result<(), ClaudePluginPackageContractError> {
    if value.is_empty()
        || value.len() > MAX_PATH_BYTES
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(ClaudePluginPackageContractError::InvalidSourcePath);
    }

    let mut component_count = 0usize;
    for component in value.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(ClaudePluginPackageContractError::InvalidSourcePath);
        }
        component_count += 1;
    }
    if component_count == 0 {
        return Err(ClaudePluginPackageContractError::InvalidSourcePath);
    }
    Ok(())
}

fn validate_probe_codes(probe_codes: &[String]) -> Result<(), ClaudePluginPackageContractError> {
    if probe_codes.is_empty() || probe_codes.len() > MAX_PROBE_COUNT {
        return Err(ClaudePluginPackageContractError::InvalidProbeSet);
    }

    let allowed: BTreeSet<&str> = ALLOWED_PROBE_CODES.iter().copied().collect();
    let mut unique = BTreeSet::new();
    for probe_code in probe_codes {
        if !allowed.contains(probe_code.as_str()) || !unique.insert(probe_code.as_str()) {
            return Err(ClaudePluginPackageContractError::InvalidProbeSet);
        }
    }
    Ok(())
}

fn validate_resource(
    field_name: &'static str,
    value: u64,
    maximum: u64,
) -> Result<(), ClaudePluginPackageContractError> {
    if value == 0 || value > maximum {
        return Err(ClaudePluginPackageContractError::InvalidResourceBudget { field_name });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ClaudePluginPackageAnalysisRequest {
        ClaudePluginPackageAnalysisRequest {
            schema_version: "1.0.0".to_owned(),
            request_id: "request_1".to_owned(),
            profile: CLAUDE_PLUGIN_PACKAGE_ANALYSIS_PROFILE.to_owned(),
            catalog_repository: "owner/catalog".to_owned(),
            catalog_commit_sha: "1".repeat(40),
            marketplace_blob_sha: "2".repeat(40),
            marketplace_entry_sha256: "a".repeat(64),
            plugin_name: "fixture".to_owned(),
            plugin_version: "1.0.0".to_owned(),
            source_repository: "owner/source".to_owned(),
            source_commit_sha: "3".repeat(40),
            source_path: "fixtures/plugin".to_owned(),
            artifact_sha256: "b".repeat(64),
            appguardrail_scan_receipt_reference: "scan_1".to_owned(),
            analysis_profile_id: "claude_plugin_package_analysis_v1".to_owned(),
            analysis_profile_sha256: "c".repeat(64),
            requested_probe_codes: vec!["package_inventory".to_owned()],
            maximum_cpu_milliseconds: 30_000,
            maximum_memory_bytes: 268_435_456,
            maximum_process_count: 32,
            maximum_disk_bytes: 536_870_912,
            maximum_output_bytes: 65_536,
            maximum_wall_time: 60,
            network_policy_reference: "network_deny_v1".to_owned(),
            filesystem_policy_reference: "filesystem_ephemeral_v1".to_owned(),
        }
    }

    #[test]
    fn canonical_request_is_valid() {
        assert_eq!(request().validate(), Ok(()));
    }

    #[test]
    fn every_probe_code_is_admitted_individually() {
        for probe_code in ALLOWED_PROBE_CODES {
            let mut candidate = request();
            candidate.requested_probe_codes = vec![(*probe_code).to_owned()];
            assert_eq!(candidate.validate(), Ok(()), "{probe_code}");
        }
    }

    #[test]
    fn every_resource_ceiling_is_inclusive() {
        let mut candidate = request();
        candidate.maximum_cpu_milliseconds = MAX_CPU_MILLISECONDS;
        candidate.maximum_memory_bytes = MAX_MEMORY_BYTES;
        candidate.maximum_process_count = MAX_PROCESS_COUNT;
        candidate.maximum_disk_bytes = MAX_DISK_BYTES;
        candidate.maximum_output_bytes = MAX_OUTPUT_BYTES;
        candidate.maximum_wall_time = MAX_WALL_TIME_SECONDS;
        assert_eq!(candidate.validate(), Ok(()));
    }
}
