//! Public-API acceptance for the synthetic, offline quickstart.

#[path = "../examples/artifact_binding_quickstart.rs"]
mod quickstart;

use quarantine_sandbox_runtime::{EvidenceBundle, RuntimeDisposition, verify_artifact_binding};

#[test]
fn quickstart_emits_a_complete_bundle_and_keeps_binding_separate_from_authority() {
    let mut output = Vec::new();
    quickstart::run(&mut output).expect("synthetic public API quickstart must succeed");
    let value: serde_json::Value = serde_json::from_slice(&output).expect("JSON output");
    assert_eq!(value["synthetic_fixture"], true);
    assert_eq!(value["evidence_scope"], "offline_artifact_consistency_only");
    assert_eq!(value["release_ready"], false);
    assert_eq!(value["operator_authenticated"], false);
    assert_eq!(value["matched_report"]["outcome"], "matched");
    assert_eq!(value["modified_bytes_report"]["outcome"], "mismatched");
    assert_eq!(value["empty_input_rejected"], true);
    assert_eq!(value["invalid_bundle_rejected"], true);
    let bundle: EvidenceBundle = serde_json::from_value(value["bundle"].clone())
        .expect("complete evidence envelope, not a projection");
    assert_eq!(bundle.validate(), Ok(()));
    assert_eq!(bundle.disposition, RuntimeDisposition::Inconclusive);
    assert!(bundle.consumer_verdict_required);
    assert!(!bundle.runtime.dynamic_execution_performed);
    assert!(!bundle.runtime.network_access_performed);
    assert!(
        bundle
            .limitations
            .iter()
            .any(|s| s == "dynamic_analysis_not_configured")
    );
    let report = verify_artifact_binding(&bundle, quickstart::SYNTHETIC_BYTES)
        .expect("consumer can independently replay against held bytes");
    assert!(report.is_matched());
    assert_eq!(
        serde_json::to_value(report).expect("report JSON"),
        value["matched_report"]
    );
}
