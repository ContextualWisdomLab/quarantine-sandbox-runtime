//! RED contract for the credential-free Claude plugin package analysis profile.
//!
//! The generic artifact-analysis request currently supports only the foundation
//! static/dynamic profile codes and rejects the immutable package-analysis
//! fields below. Issue #17 requires a dedicated, bounded profile contract before
//! any plugin bytes can reach isolated execution.

use quarantine_sandbox_runtime::AnalysisRequest;
use serde_json::Value;

fn valid_request() -> Value {
    serde_json::json!({
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
    })
}

fn validate_request(request: Value) -> Result<(), String> {
    let parsed = serde_json::from_value::<AnalysisRequest>(request).map_err(|error| error.to_string())?;
    parsed.validate().map_err(|error| error.to_string())
}

#[test]
fn immutable_claude_plugin_package_request_is_admitted_and_strictly_validated() {
    let result = validate_request(valid_request());

    assert!(
        result.is_ok(),
        "the public request contract must admit and validate the immutable, bounded Claude plugin package profile before runtime implementation is added: {result:?}"
    );
}

#[test]
fn claude_plugin_package_request_rejects_mutable_or_malformed_identity() {
    for (field, invalid_value) in [
        ("catalog_commit_sha", serde_json::json!("main")),
        ("source_commit_sha", serde_json::json!("release/latest")),
        ("marketplace_entry_sha256", serde_json::json!("abc123")),
        ("artifact_sha256", serde_json::json!("ABCDEF")),
        ("analysis_profile_sha256", serde_json::json!("0")),
    ] {
        let mut request = valid_request();
        request[field] = invalid_value;
        assert!(
            validate_request(request).is_err(),
            "{field} must be immutable and canonical before execution"
        );
    }
}

#[test]
fn claude_plugin_package_request_rejects_duplicate_probes_and_unbounded_resources() {
    let mut duplicate_probe = valid_request();
    duplicate_probe["requested_probe_codes"] = serde_json::json!([
        "package_inventory",
        "package_inventory"
    ]);
    assert!(
        validate_request(duplicate_probe).is_err(),
        "probe codes are a bounded set, not a caller-controlled repeated command list"
    );

    for field in [
        "maximum_cpu_milliseconds",
        "maximum_memory_bytes",
        "maximum_process_count",
        "maximum_disk_bytes",
        "maximum_output_bytes",
        "maximum_wall_time",
    ] {
        let mut request = valid_request();
        request[field] = serde_json::json!(0);
        assert!(
            validate_request(request).is_err(),
            "{field} must carry a positive finite execution budget"
        );
    }
}

#[test]
fn claude_plugin_package_request_rejects_control_text_in_execution_relevant_fields() {
    for field in [
        "plugin_name",
        "plugin_version",
        "source_path",
        "appguardrail_scan_receipt_reference",
        "network_policy_reference",
        "filesystem_policy_reference",
    ] {
        let mut request = valid_request();
        request[field] = serde_json::json!("safe\u{0000}unsafe");
        assert!(
            validate_request(request).is_err(),
            "{field} must reject control characters before reaching runtime logic"
        );
    }
}
