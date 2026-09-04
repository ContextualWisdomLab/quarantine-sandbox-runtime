//! RED contract for the credential-free Claude plugin package analysis profile.
//!
//! The generic artifact-analysis request currently supports only the foundation
//! static/dynamic profile codes and rejects the immutable package-analysis
//! fields below. Issue #17 requires a dedicated, bounded profile contract before
//! any plugin bytes can reach isolated execution.

use quarantine_sandbox_runtime::AnalysisRequest;

#[test]
fn immutable_claude_plugin_package_request_is_admitted_by_the_public_contract() {
    let request = serde_json::json!({
        "schema_version": "1.0.0",
        "request_id": "plugin-analysis-001",
        "profile": "claude_plugin_package_analysis",
        "catalog_repository": "anthropics/claude-plugins-community",
        "catalog_commit_sha": "1111111111111111111111111111111111111111",
        "marketplace_blob_sha": "2222222222222222222222222222222222222222",
        "marketplace_entry_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "plugin_name": "purpose-built-safe-fixture",
        "plugin_version": "1.0.0",
        "source_repository": "ContextualWisdomLab/quarantine-sandbox-runtime",
        "source_commit_sha": "3333333333333333333333333333333333333333",
        "source_path": "tests/fixtures/claude-plugin-safe",
        "artifact_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "appguardrail_scan_receipt_reference": "scan_receipt_001",
        "analysis_profile_id": "claude_plugin_package_analysis_v1",
        "analysis_profile_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "requested_probe_codes": [
            "package_inventory",
            "manifest_consistency",
            "hook_discovery",
            "mcp_stdio_schema_enumeration",
            "cleanup_recovery_probe"
        ],
        "maximum_cpu_milliseconds": 30_000,
        "maximum_memory_bytes": 268_435_456,
        "maximum_process_count": 32,
        "maximum_disk_bytes": 536_870_912,
        "maximum_output_bytes": 65_536,
        "maximum_wall_time": 60,
        "network_policy_reference": "network_policy_deny_default_v1",
        "filesystem_policy_reference": "filesystem_policy_ephemeral_v1"
    });

    let parsed = serde_json::from_value::<AnalysisRequest>(request);

    assert!(
        parsed.is_ok(),
        "the public request contract must admit the immutable, bounded Claude plugin package profile before runtime implementation is added: {parsed:?}"
    );
}
