//! Scope-to-target binding contract for self-hosted runner reachability attestation.
//!
//! The pre-checkout guard is defense in depth after GitHub has assigned the runner. Real
//! pre-lease reachability evidence remains owned by linux-cluster-ops while the runner is offline.

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
            },
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

#[derive(Debug, Eq, PartialEq)]
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
            let port = port_text.parse::<u16>().ok()?;
            return Some(Endpoint {
                host,
                port: Some(port),
            });
        }
    }

    Some(Endpoint {
        host: target,
        port: None,
    })
}

fn host_boundary_literal_ipv4(host: &str) -> bool {
    host.parse::<Ipv4Addr>().is_ok_and(|address| {
        !address.is_loopback() && (address.is_private() || address.is_link_local())
    })
}

fn target_matches_scope(required_scope: &str, target: &str) -> bool {
    let Some(endpoint) = parse_endpoint(target) else {
        return false;
    };

    match required_scope {
        "192.168.0.0/16" => {
            endpoint.port.is_none()
                && endpoint
                    .host
                    .parse::<Ipv4Addr>()
                    .is_ok_and(|address| address.octets()[..2] == [192, 168])
        }
        "10.0.0.0/8" => {
            endpoint.port.is_none()
                && endpoint
                    .host
                    .parse::<Ipv4Addr>()
                    .is_ok_and(|address| address.octets()[0] == 10)
        }
        "172.16.0.0/12" => {
            endpoint.port.is_none()
                && endpoint.host.parse::<Ipv4Addr>().is_ok_and(|address| {
                    let octets = address.octets();
                    octets[0] == 172 && (16..=31).contains(&octets[1])
                })
        }
        "6379" => endpoint.port == Some(6379) && host_boundary_literal_ipv4(endpoint.host),
        "5432" => endpoint.port == Some(5432) && host_boundary_literal_ipv4(endpoint.host),
        "DNS" => endpoint.port == Some(53) && host_boundary_literal_ipv4(endpoint.host),
        _ => false,
    }
}

#[test]
fn scope_labels_cannot_substitute_for_actual_probe_targets() {
    let label_only = literal_probe_call(
        r#"probe_forbidden_endpoint "10.0.0.0/8" "example.com:443""#,
    )
    .expect("literal helper call must parse");
    assert!(!target_matches_scope("10.0.0.0/8", &label_only.1));

    let redis_wrong_port = literal_probe_call(
        r#"probe_forbidden_endpoint "Redis 6379" "127.0.0.1:6380""#,
    )
    .expect("literal helper call must parse");
    assert!(!target_matches_scope("6379", &redis_wrong_port.1));

    let postgres_public = literal_probe_call(
        r#"probe_forbidden_endpoint "PostgreSQL 5432" "example.com:5432""#,
    )
    .expect("literal helper call must parse");
    assert!(!target_matches_scope("5432", &postgres_public.1));

    assert!(!target_matches_scope("10.0.0.0/8", "10.23.4.5:443"));
    assert!(!target_matches_scope(
        "192.168.0.0/16",
        "192.168.50.1:443"
    ));
    assert!(!target_matches_scope("6379", "127.0.0.1:6379"));
    assert!(!target_matches_scope("5432", "127.0.0.1:5432"));
    assert!(!target_matches_scope("DNS", "127.0.0.1:53"));
}

#[test]
fn required_scopes_accept_only_literal_semantically_bound_targets() {
    for (scope, target) in [
        ("192.168.0.0/16", "192.168.50.1"),
        ("10.0.0.0/8", "10.23.4.5"),
        ("172.16.0.0/12", "172.31.255.2"),
        ("6379", "10.0.0.1:6379"),
        ("5432", "192.168.50.1:5432"),
        ("DNS", "169.254.1.53:53"),
    ] {
        assert!(
            target_matches_scope(scope, target),
            "{target} must satisfy {scope}"
        );
    }

    assert!(!target_matches_scope("172.16.0.0/12", "172.32.0.1"));
    assert!(literal_probe_call(r#"probe_forbidden_endpoint "10.0.0.0/8" "$TARGET""#).is_none());
}

#[test]
fn positive_lsm_attestation_binds_each_required_scope_to_the_actual_literal_target() {
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
                scope.contains(required_scope) && target_matches_scope(required_scope, target)
            }),
            "{required_scope} must be bound to the literal target operand actually consumed by probe_forbidden_endpoint; a scope label paired with an unrelated target is not evidence"
        );
    }
}
