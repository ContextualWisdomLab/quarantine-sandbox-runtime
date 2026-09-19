//! Probe-target semantics contract for self-hosted runner reachability attestation.
//!
//! A failed probe is evidence only when the operand is meaningful for the claimed boundary.
//! A closed arbitrary TCP port inside an RFC1918 range does not prove the range is unreachable,
//! and an unresolved symbolic host alias does not prove a host service is unreachable.

use std::{fs, net::Ipv4Addr};

fn section_end(workflow: &str, body_start: usize) -> usize {
    let remainder = &workflow[body_start..];
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n');
        let sibling_header = line_without_newline.starts_with("  ")
            && !line_without_newline.starts_with("   ")
            && !line_without_newline.trim().is_empty();
        let top_level = !line_without_newline.is_empty() && !line_without_newline.starts_with(' ');
        if sibling_header || top_level {
            return body_start + offset;
        }
        offset += line.len();
    }
    workflow.len()
}

fn job_section<'a>(workflow: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = workflow
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI job {job_name}"));
    let body_start = start + marker.len();
    let end = section_end(workflow, body_start);
    &workflow[start..end]
}

fn named_step_section<'a>(job: &'a str, step_name: &str) -> &'a str {
    let marker = format!("\n      - name: {step_name}\n");
    let start = job
        .find(&marker)
        .unwrap_or_else(|| panic!("missing CI step {step_name}"));
    let body_start = start + marker.len();
    let remainder = &job[body_start..];
    let end = remainder
        .find("\n      - ")
        .map_or(job.len(), |offset| body_start + offset);
    &job[start..end]
}

fn inline_script(step: &str) -> &str {
    let marker = step
        .find("\n        run: |")
        .or_else(|| step.find("\n        run: >-"))
        .unwrap_or_else(|| panic!("attestation must be an inline run step"));
    let start = step[marker + 1..]
        .find('\n')
        .map(|offset| marker + 1 + offset + 1)
        .unwrap_or(step.len());
    &step[start..]
}

fn code_before_inline_comment(line: &str) -> &str {
    line.split_once(" #")
        .map_or(line, |(code, _comment)| code)
        .trim_end()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuoteState {
    Unquoted,
    Single,
    Double,
}

fn literal_shell_words(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = QuoteState::Unquoted;
    let mut escaped = false;

    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match quote {
            QuoteState::Unquoted => match character {
                '\\' => escaped = true,
                '\'' => quote = QuoteState::Single,
                '"' => quote = QuoteState::Double,
                '$' | '`' | ';' | '|' | '&' | '<' | '>' => return None,
                value if value.is_ascii_whitespace() => {
                    if !current.is_empty() {
                        words.push(std::mem::take(&mut current));
                    }
                }
                value => current.push(value),
            },
            QuoteState::Single => {
                if character == '\'' {
                    quote = QuoteState::Unquoted;
                } else {
                    current.push(character);
                }
            }
            QuoteState::Double => match character {
                '"' => quote = QuoteState::Unquoted,
                '\\' => escaped = true,
                '$' | '`' => return None,
                value => current.push(value),
            },
        }
    }

    if escaped || quote != QuoteState::Unquoted {
        return None;
    }
    if !current.is_empty() {
        words.push(current);
    }
    Some(words)
}

fn literal_probe_call(line: &str) -> Option<(String, String)> {
    let code = code_before_inline_comment(line).trim();
    let code = code.strip_prefix("if ").unwrap_or(code).trim_start();
    let code = code.strip_prefix("! ").unwrap_or(code).trim_start();
    let code = code.strip_suffix("; then").unwrap_or(code).trim_end();
    let arguments = code.strip_prefix("probe_forbidden_endpoint ")?;
    let words = literal_shell_words(arguments)?;
    if words.len() != 2 {
        return None;
    }
    Some((words[0].clone(), words[1].clone()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Endpoint<'a> {
    host: &'a str,
    port: Option<u16>,
}

fn parse_endpoint(target: &str) -> Option<Endpoint<'_>> {
    if target.is_empty() || target.chars().any(char::is_whitespace) {
        return None;
    }

    if let Some((host, port_text)) = target.rsplit_once(':') {
        if !host.is_empty()
            && !port_text.is_empty()
            && port_text.chars().all(|character| character.is_ascii_digit())
        {
            return Some(Endpoint {
                host,
                port: Some(port_text.parse::<u16>().ok()?),
            });
        }
    }

    Some(Endpoint {
        host: target,
        port: None,
    })
}

fn private_literal_ipv4(host: &str) -> bool {
    host.parse::<Ipv4Addr>().is_ok_and(|address| {
        address.is_private() || address.is_loopback() || address.is_link_local()
    })
}

fn host_in_scope(scope: &str, host: &str) -> bool {
    let Ok(address) = host.parse::<Ipv4Addr>() else {
        return false;
    };
    let octets = address.octets();

    match scope {
        "192.168.0.0/16" => octets[0] == 192 && octets[1] == 168,
        "10.0.0.0/8" => octets[0] == 10,
        "172.16.0.0/12" => octets[0] == 172 && (16..=31).contains(&octets[1]),
        _ => false,
    }
}

fn target_has_meaningful_probe_semantics(scope: &str, target: &str) -> bool {
    let Some(endpoint) = parse_endpoint(target) else {
        return false;
    };

    match scope {
        "192.168.0.0/16" | "10.0.0.0/8" | "172.16.0.0/12" => {
            endpoint.port.is_none() && host_in_scope(scope, endpoint.host)
        }
        "6379" => endpoint.port == Some(6379) && private_literal_ipv4(endpoint.host),
        "5432" => endpoint.port == Some(5432) && private_literal_ipv4(endpoint.host),
        "DNS" => endpoint.port == Some(53) && private_literal_ipv4(endpoint.host),
        _ => false,
    }
}

#[test]
fn closed_service_ports_and_unresolved_aliases_cannot_masquerade_as_network_denial() {
    assert!(!target_has_meaningful_probe_semantics(
        "10.0.0.0/8",
        "10.23.4.5:65535"
    ));
    assert!(!target_has_meaningful_probe_semantics(
        "192.168.0.0/16",
        "192.168.50.1:1"
    ));
    assert!(!target_has_meaningful_probe_semantics(
        "6379",
        "host-gateway:6379"
    ));
    assert!(!target_has_meaningful_probe_semantics(
        "5432",
        "host.containers.internal:5432"
    ));
    assert!(!target_has_meaningful_probe_semantics("DNS", "localhost:53"));
}

#[test]
fn network_scopes_require_host_reachability_targets_and_services_require_literal_private_addresses() {
    for (scope, target) in [
        ("192.168.0.0/16", "192.168.50.1"),
        ("10.0.0.0/8", "10.23.4.5"),
        ("172.16.0.0/12", "172.31.255.2"),
        ("6379", "127.0.0.1:6379"),
        ("5432", "10.0.0.1:5432"),
        ("DNS", "10.0.0.53:53"),
    ] {
        assert!(
            target_has_meaningful_probe_semantics(scope, target),
            "{target} must remain an admissible target for {scope}"
        );
    }
}

#[test]
fn positive_lsm_attestation_does_not_reduce_network_denial_to_a_closed_port_probe() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let job = job_section(&workflow, "podman-e2e-positive-lsm");
    let gate = named_step_section(job, "Attest self-hosted runner LAN and host-service denial");
    let script = inline_script(gate);
    let direct_calls: Vec<(String, String)> = script.lines().filter_map(literal_probe_call).collect();

    for required_scope in [
        "192.168.0.0/16",
        "10.0.0.0/8",
        "172.16.0.0/12",
        "6379",
        "5432",
        "DNS",
    ] {
        assert!(
            direct_calls.iter().any(|(scope, target)| {
                scope.contains(required_scope)
                    && target_has_meaningful_probe_semantics(required_scope, target)
            }),
            "{required_scope} must use a semantically meaningful probe operand: RFC1918 range checks require a host target without an arbitrary service port, while service checks require the exact port on a literal private/loopback/link-local IPv4 address"
        );
    }
}
