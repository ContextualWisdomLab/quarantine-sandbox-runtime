//! RED: application-service network authority must come from creation-bound evidence.
//!
//! Podman's network-create CLI prints the created network name, while Podman's network-create
//! implementation has already assigned an immutable backend ID and synchronously emits that ID
//! in the network-create event before the CLI returns. A later public-name inspect is therefore
//! not an acceptable source of private attachment or cleanup authority when bounded creation
//! history can provide the ID of the object created by this invocation.

#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use quarantine_sandbox_runtime::{
    ApplicationServiceRequest, IsolationPolicy, ResourceRequest, RootlessPodmanAdapter,
    ServiceProtocol,
};

const STARTED_AT_EPOCH_SECONDS: u64 = 1_780_004_700;
const CREATED_NETWORK_ID: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REPLACEMENT_NETWORK_ID: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
static NEXT_TEMP_PATH_ID: AtomicU64 = AtomicU64::new(0);

fn temporary_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock must be after the Unix epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_PATH_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "qsr-network-creation-receipt-red-{name}-{}-{nanos}-{unique_id}",
        std::process::id()
    ))
}

fn policy() -> IsolationPolicy {
    IsolationPolicy {
        policy_id: "network_creation_receipt_red_v1".to_owned(),
        maximum_memory_bytes: 256 * 1024 * 1024,
        maximum_cpu_millicores: 500,
        maximum_processes: 32,
        maximum_lease_seconds: 60,
        maximum_tmpfs_bytes: 32 * 1024 * 1024,
        readiness_timeout_millis: 500,
        readiness_poll_interval_millis: 10,
        shutdown_grace_seconds: 1,
        run_as_user_id: 65_532,
        run_as_group_id: 65_532,
    }
}

fn request() -> ApplicationServiceRequest {
    ApplicationServiceRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "network_creation_receipt_red".to_owned(),
        image_reference: format!("localhost/cwl/tool@sha256:{}", "f".repeat(64)),
        container_port: 8_080,
        protocol: ServiceProtocol::Tcp,
        command: vec!["serve".to_owned()],
        resources: ResourceRequest {
            memory_bytes: 128 * 1024 * 1024,
            cpu_millicores: 250,
            maximum_processes: 16,
            lease_seconds: 30,
            tmpfs_bytes: 16 * 1024 * 1024,
        },
    }
}

fn write_fake_podman(
    log: &Path,
    network_selector_marker: &Path,
    ambiguous_history: bool,
) -> PathBuf {
    let program = temporary_path("fake-podman");
    let info = r#"{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}}}"#;
    let script = format!(
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> '{log}'
network_selector_marker='{network_selector_marker}'
created_network_id='{created_network_id}'
replacement_network_id='{replacement_network_id}'
ambiguous_history='{ambiguous_history}'
if [ "${{1:-}}" = info ]; then
  if [ "${{3:-}}" = json ]; then printf '%s\n' '{info}'; else printf 'true\n'; fi
  exit 0
fi
case "${{1:-}}:${{2:-}}" in
  network:create)
    printf '%s\n' "${{5:-}}"
    ;;
  events:*)
    created_name=$(awk '$1 == "network" && $2 == "create" {{ name=$NF }} END {{ print name }}' '{log}')
    printf '{{"ID":"%s","Network":"%s","Status":"create","Time":"2026-09-22T13:30:00Z","Type":"network"}}\n' "$created_network_id" "$created_name"
    if [ "$ambiguous_history" = true ]; then
      printf '{{"ID":"%s","Network":"%s","Status":"create","Time":"2026-09-22T13:30:00Z","Type":"network"}}\n' "$replacement_network_id" "$created_name"
    fi
    ;;
  network:inspect)
    selector=${{5:-}}
    if [ "$selector" = "$created_network_id" ]; then
      printf '[{{"name":"created-object","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$created_network_id"
    else
      printf '[{{"name":"%s","id":"%s","internal":true,"dns_enabled":false,"containers":{{}}}}]\n' "$selector" "$replacement_network_id"
    fi
    ;;
  create:--name)
    previous=''
    selected=''
    for argument in "$@"; do
      if [ "$previous" = '--network' ]; then selected=$argument; break; fi
      previous=$argument
    done
    printf '%s\n' "$selected" > "$network_selector_marker"
    exit 97
    ;;
  network:rm)
    :
    ;;
  *)
    exit 91
    ;;
esac
"#,
        log = log.display(),
        network_selector_marker = network_selector_marker.display(),
        created_network_id = CREATED_NETWORK_ID,
        replacement_network_id = REPLACEMENT_NETWORK_ID,
        ambiguous_history = ambiguous_history,
        info = info,
    );

    fs::write(&program, script).expect("fake Podman must be writable");
    let mut permissions = fs::metadata(&program)
        .expect("fake Podman metadata must exist")
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&program, permissions).expect("fake Podman must be executable");
    program
}

fn option_values<'a>(arguments: &'a [&'a str], option: &str) -> Vec<&'a str> {
    let assignment_prefix = format!("{option}=");
    let mut occurrences = 0;
    let mut values = Vec::new();

    for (index, argument) in arguments.iter().enumerate() {
        if *argument == option {
            occurrences += 1;
            if let Some(candidate) = arguments
                .get(index + 1)
                .copied()
                .filter(|candidate| !candidate.is_empty() && !candidate.starts_with("--"))
            {
                values.push(candidate);
            }
        } else if let Some(candidate) = argument.strip_prefix(&assignment_prefix) {
            occurrences += 1;
            if !candidate.is_empty() {
                values.push(candidate);
            }
        }
    }

    assert_eq!(
        occurrences,
        values.len(),
        "every creation-history {option} occurrence must have a concrete value"
    );
    values
}

fn single_option_value<'a>(arguments: &'a [&'a str], option: &str) -> &'a str {
    let values = option_values(arguments, option);
    assert_eq!(
        values.len(),
        1,
        "creation-history option {option} must occur exactly once"
    );
    values[0]
}

fn bounded_creation_event_query(calls: &str) -> &str {
    let call_lines: Vec<&str> = calls.lines().collect();
    let network_create_index = call_lines
        .iter()
        .position(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("network creation must precede creation-history admission");
    let mut event_queries = call_lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("events "));
    let (event_query_index, event_query) = event_queries
        .next()
        .expect("network identity admission must consult Podman creation history");
    assert!(
        event_queries.next().is_none(),
        "network identity admission must use exactly one creation-history query; calls were:\n{calls}"
    );
    let event_query = *event_query;
    assert!(
        event_query_index > network_create_index,
        "creation history must be queried only after the invocation-local network is created; calls were:\n{calls}"
    );
    if let Some(network_inspect_index) = call_lines
        .iter()
        .position(|line| line.starts_with("network inspect "))
    {
        assert!(
            event_query_index < network_inspect_index,
            "creation receipt must be admitted before exact-ID network inspection; calls were:\n{calls}"
        );
    }
    if let Some(container_create_index) = call_lines
        .iter()
        .position(|line| line.starts_with("create --name "))
    {
        assert!(
            event_query_index < container_create_index,
            "creation receipt must be admitted before container creation; calls were:\n{calls}"
        );
    }

    let arguments: Vec<&str> = event_query.split_whitespace().collect();
    let since = single_option_value(&arguments, "--since");
    let until = single_option_value(&arguments, "--until");
    assert_ne!(
        since, until,
        "creation-history lower and upper bounds must be distinct; call was: {event_query}"
    );

    assert_eq!(
        single_option_value(&arguments, "--stream"),
        "false",
        "creation history must disable streaming exactly; call was: {event_query}"
    );
    assert_eq!(
        single_option_value(&arguments, "--format"),
        "json",
        "creation history must request JSON exactly; call was: {event_query}"
    );
    let filters = option_values(&arguments, "--filter");
    assert_eq!(
        filters.len(),
        2,
        "creation history must use exactly the network/create filters; call was: {event_query}"
    );
    assert!(
        filters.contains(&"type=network") && filters.contains(&"event=create"),
        "creation history must filter exactly to network/create events; call was: {event_query}"
    );

    event_query
}

#[test]
fn creation_history_id_must_win_over_later_same_name_resolution() {
    let log = temporary_path("calls");
    let network_selector_marker = temporary_path("network-selector");
    let program = write_fake_podman(&log, &network_selector_marker, false);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let selected_network = fs::read_to_string(&network_selector_marker).ok();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_selector_marker);

    let network_create = calls
        .lines()
        .find(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("the witness must reach network creation");
    let created_name = network_create
        .split_whitespace()
        .last()
        .expect("network creation must include a generated correlation name");
    let public_name_lookup = format!("network inspect --format json {created_name}");

    assert!(result.is_err(), "the controlled container-create failure must surface");
    bounded_creation_event_query(&calls);
    assert!(
        !calls.lines().any(|line| line == public_name_lookup),
        "a post-create public-name lookup must not mint private network authority; calls were:\n{calls}"
    );
    assert_eq!(
        selected_network.as_deref().map(str::trim),
        Some(CREATED_NETWORK_ID),
        "container creation must bind the ID from the creation-bound receipt"
    );
    assert!(
        calls
            .lines()
            .any(|line| line == format!("network rm {CREATED_NETWORK_ID}")),
        "partial-launch cleanup must retain the same creation-bound ID; calls were:\n{calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|line| line == format!("network rm {REPLACEMENT_NETWORK_ID}")),
        "a same-name replacement must never become destructive authority; calls were:\n{calls}"
    );
}

#[test]
fn ambiguous_creation_history_must_fail_before_private_authority_is_minted() {
    let log = temporary_path("ambiguous-calls");
    let network_selector_marker = temporary_path("ambiguous-network-selector");
    let program = write_fake_podman(&log, &network_selector_marker, true);
    let adapter = RootlessPodmanAdapter::new(program.clone());

    let result = adapter.launch_at(&request(), &policy(), STARTED_AT_EPOCH_SECONDS);
    let calls = fs::read_to_string(&log).expect("fake Podman calls must be recorded");
    let selected_network = fs::read_to_string(&network_selector_marker).ok();

    let _ = fs::remove_file(program);
    let _ = fs::remove_file(log);
    let _ = fs::remove_file(network_selector_marker);

    let network_create = calls
        .lines()
        .find(|line| line.starts_with("network create --internal --disable-dns qsr-net-"))
        .expect("the witness must reach network creation");
    let created_name = network_create
        .split_whitespace()
        .last()
        .expect("network creation must include a generated correlation name");
    let public_name_lookup = format!("network inspect --format json {created_name}");

    assert!(result.is_err(), "ambiguous creation history must fail closed");
    bounded_creation_event_query(&calls);
    assert!(
        !calls.lines().any(|line| line == public_name_lookup),
        "ambiguity must not fall back to a mutable public-name lookup; calls were:\n{calls}"
    );
    assert!(
        selected_network.is_none(),
        "ambiguous matching creation events must stop before container creation"
    );
    assert!(
        !calls.lines().any(|line| line.starts_with("network rm ")),
        "ambiguous creation history does not establish safe destructive authority; calls were:\n{calls}"
    );
}
