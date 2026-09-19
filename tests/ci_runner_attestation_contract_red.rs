//! Causal-shape contract for the self-hosted runner reachability attestation.
//!
//! The workflow guard is defense in depth after a runner has been assigned. It must not be
//! confused with the independent pre-lease host proof owned by linux-cluster-ops.

use std::fs;

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

fn shell_function_body<'a>(script: &'a str, function_name: &str) -> &'a str {
    let declaration = format!("{function_name}() {{");
    let start = script
        .find(&declaration)
        .unwrap_or_else(|| panic!("missing shell helper {function_name}"));
    let body_start = start + declaration.len();
    let remainder = &script[body_start..];
    let end = remainder
        .lines()
        .scan(0usize, |offset, line| {
            let current = *offset;
            *offset += line.len() + 1;
            Some((current, line))
        })
        .find_map(|(offset, line)| (line.trim() == "}").then_some(body_start + offset))
        .unwrap_or_else(|| panic!("unterminated shell helper {function_name}"));
    &script[body_start..end]
}

fn contains_probe_token(command: &str) -> bool {
    [" nc ", " ncat ", " curl ", " dig ", " getent ", " ping "]
        .iter()
        .any(|needle| command.contains(needle))
        || command.contains("/dev/tcp/")
}

fn is_executable_probe_line(line: &str) -> bool {
    let mut command = line.trim();
    if let Some(rest) = command.strip_prefix("if ") {
        command = rest.trim_start();
    }
    if let Some(rest) = command.strip_prefix("! ") {
        command = rest.trim_start();
    }

    ["nc ", "ncat ", "curl ", "dig ", "getent ", "ping "]
        .iter()
        .any(|prefix| command.starts_with(prefix))
        || command.starts_with("timeout ") && contains_probe_token(command)
        || command.starts_with("bash -c ") && command.contains("/dev/tcp/")
}

fn shell_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn caller_target_tokens(helper: &str) -> Vec<String> {
    let mut tokens = vec!["$2".to_owned(), "${2}".to_owned()];

    for line in helper.lines().map(str::trim) {
        let assignment = line.strip_prefix("local ").unwrap_or(line).trim_start();
        let Some((name, value)) = assignment.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if !shell_identifier(name) {
            continue;
        }
        if ["$2", "${2}", "\"$2\"", "\"${2}\""]
            .iter()
            .any(|candidate| value == *candidate)
        {
            tokens.push(format!("${name}"));
            tokens.push(format!("${{{name}}}"));
        }
    }

    tokens
}

fn code_before_inline_comment(line: &str) -> &str {
    line.split_once(" #")
        .map_or(line, |(code, _comment)| code)
        .trim_end()
}

#[test]
fn positive_lsm_attestation_binds_each_scope_to_the_executed_probe_helper() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .expect("CI workflow must be readable from the repository root");
    let job = job_section(&workflow, "podman-e2e-positive-lsm");
    let gate_name = "Attest self-hosted runner LAN and host-service denial";
    let gate = named_step_section(job, gate_name);
    let script = inline_script(gate);
    let helper = shell_function_body(script, "probe_forbidden_endpoint");

    assert!(
        helper.contains("$1") && helper.contains("$2"),
        "the probe helper must consume caller-supplied scope/endpoint arguments rather than probe one unrelated fixed target"
    );
    assert!(
        helper.lines().any(is_executable_probe_line),
        "the probe helper body itself must execute a concrete network or DNS probe"
    );

    let caller_target_tokens = caller_target_tokens(helper);
    assert!(
        helper.lines().any(|line| {
            line.trim_start().starts_with("if ")
                && is_executable_probe_line(line)
                && caller_target_tokens.iter().any(|token| line.contains(token))
        }),
        "the helper must branch directly on a concrete probe whose target operand is caller-supplied, not on an unrelated fixed target while merely logging caller arguments"
    );
    assert!(
        helper.lines().any(|line| line.contains("exit 1")),
        "a reachable forbidden endpoint must fail from inside the probe helper"
    );

    let evidence_assignment = script.lines().map(str::trim).any(|line| {
        line.starts_with("evidence_file=") && line.to_ascii_lowercase().contains(".json")
    });
    assert!(
        evidence_assignment,
        "the attestation script must bind one machine-readable JSON evidence file"
    );
    assert!(
        helper.lines().any(|line| {
            (line.contains("$evidence_file") || line.contains("${evidence_file}"))
                && (line.contains(">>") || line.contains('>'))
        }),
        "the executed helper must append or write each observed probe result to the JSON evidence file"
    );

    let direct_calls: Vec<&str> = script
        .lines()
        .map(str::trim)
        .map(code_before_inline_comment)
        .filter(|line| {
            line.starts_with("probe_forbidden_endpoint ")
                || line.starts_with("if probe_forbidden_endpoint ")
        })
        .collect();

    for required_scope in [
        "192.168.0.0/16",
        "10.0.0.0/8",
        "172.16.0.0/12",
        "6379",
        "5432",
        "DNS",
    ] {
        assert!(
            direct_calls.iter().any(|line| line.contains(required_scope)),
            "{required_scope} must be passed as executable helper input before any inline comment, not merely mentioned in output/configuration/comment text"
        );
    }
}
