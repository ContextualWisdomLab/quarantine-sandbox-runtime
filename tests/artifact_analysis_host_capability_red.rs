//! Regression: static analyzers must not inherit runtime-host capabilities.
//!
//! A public in-process `StaticAnalyzer` can currently open host sockets while
//! `AnalysisEngine` later attests that no network access occurred. The static
//! profile must either reject such an analyzer configuration before invocation
//! or execute analyzers behind an enforceable capability-denying boundary.

use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use quarantine_sandbox_runtime::{
    AnalysisEngine, AnalysisProfile, AnalysisRequest, AnalyzerFailure, AnalyzerFinding,
    CONTRACT_SCHEMA_VERSION, IngestedArtifact, IngestionPolicy, StaticAnalyzer,
};

struct HostNetworkAnalyzer {
    target: SocketAddr,
    connected: Arc<AtomicBool>,
}

impl StaticAnalyzer for HostNetworkAnalyzer {
    fn analyzer_id(&self) -> &'static str {
        "host_network_red_analyzer"
    }

    fn analyze(
        &self,
        _artifact: &IngestedArtifact,
    ) -> Result<Vec<AnalyzerFinding>, AnalyzerFailure> {
        if TcpStream::connect_timeout(&self.target, Duration::from_millis(250)).is_ok() {
            self.connected.store(true, Ordering::SeqCst);
        }
        Ok(Vec::new())
    }
}

fn request() -> AnalysisRequest {
    AnalysisRequest {
        schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),
        request_id: "artifact_analysis_host_capability_red".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: None,
    }
}

#[test]
fn static_analyzer_cannot_use_runtime_host_network_while_no_network_is_attested() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback listener must bind");
    let target = listener
        .local_addr()
        .expect("loopback listener must expose its address");
    let connected = Arc::new(AtomicBool::new(false));
    let analyzer = HostNetworkAnalyzer {
        target,
        connected: Arc::clone(&connected),
    };
    let engine = AnalysisEngine::new(
        IngestionPolicy::default(),
        "static_worker_policy_v1",
        "artifact_worker_red",
        vec![Box::new(analyzer)],
    )
    .expect("otherwise-valid analyzer configuration must construct");

    let result = engine.analyze_bytes(&request(), b"hostile artifact bytes");

    assert!(
        !connected.load(Ordering::SeqCst),
        "static analyzer reached a runtime-host network socket; the host-process trait call is not an isolation boundary"
    );

    if let Ok(bundle) = result {
        assert!(
            !bundle.runtime.network_access_performed,
            "a contained static worker must not rewrite no-network evidence as network activity"
        );
    }

    drop(listener);
}
