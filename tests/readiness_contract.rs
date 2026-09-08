use quarantine_sandbox_readiness::{
    probe_readiness, FailureKind, ProbeRequest, ReadinessOutcome,
};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn request(origin: String) -> ProbeRequest {
    ProbeRequest {
        runtime_image_digest:
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        instance_id: "strix-caido-42".into(),
        origin,
        method: "POST".into(),
        path: "/graphql".into(),
        request_body: br#"{"query":"mutation { loginAsGuest { token } }"}"#.to_vec(),
        expected_status: 200,
        expected_body_substring: Some("\"token\":\"guest\"".into()),
        timeout: Duration::from_secs(2),
        poll_interval: Duration::from_millis(10),
    }
}

fn serve_after(delay: Duration, response: &'static [u8]) -> (String, thread::JoinHandle<()>) {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let handle = thread::spawn(move || {
        thread::sleep(delay);
        let listener = TcpListener::bind(address).unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        let mut request_bytes = Vec::new();
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        let _ = stream.read_to_end(&mut request_bytes);
        assert!(String::from_utf8_lossy(&request_bytes).starts_with("POST /graphql HTTP/1.1"));
        stream.write_all(response).unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut eof = [0_u8; 1];
        assert_eq!(stream.read(&mut eof).unwrap(), 0, "client must close its connection");
    });
    (format!("http://{address}"), handle)
}

#[test]
fn delayed_loopback_handshake_emits_digest_bound_ready_receipt() {
    let (origin, server) = serve_after(
        Duration::from_millis(75),
        b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{"token":"guest"}",
    );
    let cancelled = AtomicBool::new(false);
    let receipt = probe_readiness(&request(origin), || cancelled.load(Ordering::Relaxed));
    server.join().unwrap();

    assert_eq!(receipt.schema_version, "qsr.readiness.v1");
    assert_eq!(receipt.outcome, ReadinessOutcome::Ready);
    assert_eq!(receipt.runtime_image_digest, request("http://127.0.0.1:1".into()).runtime_image_digest);
    assert_eq!(receipt.instance_id, "strix-caido-42");
    assert!(receipt.attempts > 1);
    assert!(receipt.completed_monotonic_ms >= receipt.started_monotonic_ms);
    assert_eq!(receipt.http_status, Some(200));
    assert_eq!(receipt.failure_kind, None);
    assert_eq!(receipt.request_sha256.len(), 64);
    assert_eq!(receipt.response_sha256.as_deref().map(str::len), Some(64));
}

#[test]
fn never_listening_proxy_fails_closed_as_sandbox_unavailable() {
    let reservation = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = reservation.local_addr().unwrap();
    drop(reservation);
    let cancelled = AtomicBool::new(false);
    let mut req = request(format!("http://{address}"));
    req.timeout = Duration::from_millis(80);

    let receipt = probe_readiness(&req, || cancelled.load(Ordering::Relaxed));

    assert_eq!(receipt.outcome, ReadinessOutcome::Failed);
    assert_eq!(receipt.failure_kind, Some(FailureKind::SandboxUnavailable));
    assert!(receipt.http_status.is_none());
}

#[test]
fn successful_http_without_login_marker_is_not_ready() {
    let (origin, server) = serve_after(
        Duration::ZERO,
        b"HTTP/1.1 200 OK\r\nContent-Length: 14\r\nConnection: close\r\n\r\n{"token":null}",
    );
    let receipt = probe_readiness(&request(origin), || false);
    server.join().unwrap();

    assert_eq!(receipt.outcome, ReadinessOutcome::Failed);
    assert_eq!(receipt.failure_kind, Some(FailureKind::SandboxUnavailable));
    assert_eq!(receipt.http_status, Some(200));
}

#[test]
fn caller_cancellation_is_distinct_from_sandbox_failure() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancelled);
    let canceller = thread::spawn(move || {
        thread::sleep(Duration::from_millis(25));
        flag.store(true, Ordering::Relaxed);
    });
    let mut req = request("http://127.0.0.1:9".into());
    req.timeout = Duration::from_secs(1);

    let receipt = probe_readiness(&req, || cancelled.load(Ordering::Relaxed));
    canceller.join().unwrap();

    assert_eq!(receipt.outcome, ReadinessOutcome::Failed);
    assert_eq!(receipt.failure_kind, Some(FailureKind::UserCancelled));
}

#[test]
fn non_loopback_and_unpinned_runtime_identity_are_rejected() {
    let mut external = request("http://192.0.2.1:48080".into());
    let receipt = probe_readiness(&external, || false);
    assert_eq!(receipt.failure_kind, Some(FailureKind::InvalidRequest));

    external.origin = "http://127.0.0.1:48080".into();
    external.runtime_image_digest = "latest".into();
    let receipt = probe_readiness(&external, || false);
    assert_eq!(receipt.failure_kind, Some(FailureKind::InvalidRequest));
}

#[test]
fn receipt_schema_reserves_non_sandbox_terminal_causes() {
    assert_eq!(serde_json::to_string(&FailureKind::ProviderTerminated).unwrap(), ""provider_terminated"");
    assert_eq!(
        serde_json::to_string(&FailureKind::ModelCommunicationFailed).unwrap(),
        ""model_communication_failed""
    );
}
