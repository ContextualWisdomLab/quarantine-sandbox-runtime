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
        if ["$2", "${2}", "\"$2\"", "\"${2}\""].contains(&value) {
            tokens.push(format!("${name}"));
            tokens.push(format!("${{{name}}}"));
        }
    }

    tokens
}

// This recognizes a deliberately small single-line shell shape, not execution semantics.
// Unsupported compound commands/substitutions are rejected rather than treated as probes.
fn code_before_inline_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let mut word_start = true;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            word_start = false;
            continue;
        }
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('\''), _) => {}
            (_, '\\') => escaped = true,
            (None, '\'' | '"') => {
                quote = Some(character);
                word_start = false;
            }
            (None, '#') if word_start => return line[..index].trim_end(),
            (None, value) => {
                word_start = value.is_whitespace() || ";|&()<>".contains(value);
            }
            _ => {}
        }
    }
    line.trim_end()
}

fn simple_probe_words(command: &str) -> Option<Vec<&str>> {
    let mut words = Vec::new();
    let mut start = None;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in command.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('\''), _) => {}
            (_, '\\') => escaped = true,
            (None, '\'' | '"') => quote = Some(character),
            (_, '`' | '(' | ')') => return None,
            (None, ';' | '|' | '&' | '<' | '>') => return None,
            (None, value) if value.is_whitespace() => {
                if let Some(begin) = start.take() {
                    words.push(&command[begin..index]);
                }
                continue;
            }
            _ => {}
        }
        start.get_or_insert(index);
    }
    if quote.is_some() || escaped {
        return None;
    }
    if let Some(begin) = start {
        words.push(&command[begin..]);
    }
    Some(words)
}

// Trusted shape only: one-line assignments/printf/echo/exit and balanced
// if/else/fi, with the two explicit data-free/evidence redirection suffixes.
// Validate the entire helper before scanning any probe. This is not a shell
// interpreter or alias/control-flow proof; all other source shapes fail closed.
fn helper_has_supported_source(helper: &str) -> bool {
    let mut branches: Vec<bool> = Vec::new();
    for line in helper.lines() {
        let code = code_before_inline_comment(line).trim();
        if code.is_empty() {
            continue;
        }
        // No heredocs, here strings, substitutions or multiline shell tokens.
        if code.contains("<<") {
            return false;
        }
        let mut quote = None;
        let mut escaped = false;
        let mut previous = None;
        let mut characters = code.chars();
        while let Some(character) = characters.next() {
            if escaped {
                escaped = false;
                previous = None;
                continue;
            }
            match (quote, character) {
                (Some('\''), '\'') | (Some('"'), '"') => quote = None,
                (Some('\''), _) => {}
                (_, '\\') => escaped = true,
                (None, '\'' | '"') => quote = Some(character),
                (_, '`' | '(' | ')' | '}') => return false,
                (_, '{') => {
                    if previous != Some('$') {
                        return false;
                    }
                    let mut name = String::new();
                    loop {
                        match characters.next() {
                            Some('}') => break,
                            Some(value) => name.push(value),
                            None => return false,
                        }
                    }
                    if !shell_identifier(&name)
                        && !(name.len() == 1 && name.bytes().all(|value| value.is_ascii_digit()))
                    {
                        return false;
                    }
                }
                _ => {}
            }
            previous = Some(character);
        }
        if quote.is_some() || escaped {
            return false;
        }
        if code == "fi" {
            if branches.pop().is_none() {
                return false;
            }
            continue;
        }
        if code == "else" {
            match branches.last_mut() {
                Some(seen_else) if !*seen_else => *seen_else = true,
                _ => return false,
            }
            continue;
        }
        let conditional = code.strip_prefix("if ");
        let command = if let Some(condition) = conditional {
            let Some(command) = condition.strip_suffix("; then") else {
                return false;
            };
            branches.push(false);
            command
        } else {
            code
        };
        let command = command
            .trim()
            .strip_suffix(" >/dev/null 2>&1")
            .or_else(|| command.trim().strip_suffix(" >> \"$evidence_file\""))
            .unwrap_or(command.trim());
        let Some(words) = simple_probe_words(command) else {
            return false;
        };
        let Some(first) = words.first() else {
            return false;
        };
        if conditional.is_some() {
            if !["timeout", "nc", "ncat", "curl", "dig", "getent", "ping"].contains(first) {
                return false;
            }
        } else if !["printf", "echo", "exit"].contains(first) {
            let assignment = if *first == "local" && words.len() == 2 {
                words[1]
            } else if words.len() == 1 {
                first
            } else {
                return false;
            };
            if !assignment
                .split_once('=')
                .is_some_and(|(name, _)| shell_identifier(name))
            {
                return false;
            }
        }
    }
    branches.is_empty()
}

fn helper_has_caller_target_probe(helper: &str) -> bool {
    if !helper_has_supported_source(helper) {
        return false;
    }
    let tokens = caller_target_tokens(helper);
    helper.lines().any(|line| {
        let code = code_before_inline_comment(line).trim();
        let Some(condition) = code
            .strip_prefix("if ")
            .and_then(|s| s.strip_suffix("; then"))
        else {
            return false;
        };
        // Only this known, data-free redirection suffix is supported by the shape check.
        let condition = condition
            .trim()
            .strip_suffix(" >/dev/null 2>&1")
            .unwrap_or(condition.trim());
        let Some(words) = simple_probe_words(condition) else {
            return false;
        };
        let probe_index = if words.first() == Some(&"timeout") {
            if !words
                .get(1)
                .is_some_and(|duration| duration.parse::<u32>().is_ok())
            {
                return false;
            }
            2
        } else {
            0
        };
        if !words
            .get(probe_index)
            .is_some_and(|probe| ["nc", "ncat", "curl", "dig", "getent", "ping"].contains(probe))
        {
            return false;
        }
        let probe = words[probe_index];
        let mut arguments = &words[probe_index + 1..];
        // Small allowlist: unknown options may consume data instead of an endpoint.
        while let Some(option) = arguments.first().filter(|word| word.starts_with('-')) {
            let takes_number = match (probe, *option) {
                ("nc" | "ncat", "-w") | ("ping", "-c" | "-W") => true,
                ("nc" | "ncat", "-z" | "-v" | "-n" | "-vz" | "-zv")
                | ("curl", "-f" | "--fail" | "-s" | "-S" | "-sS") => false,
                _ => return false,
            };
            arguments = &arguments[1..];
            if takes_number {
                if !arguments
                    .first()
                    .is_some_and(|value| value.parse::<u32>().is_ok())
                {
                    return false;
                }
                arguments = &arguments[1..];
            }
        }
        if probe == "getent" {
            if arguments.first() != Some(&"hosts") {
                return false;
            }
            arguments = &arguments[1..];
        }
        let Some(word) = arguments.first() else {
            return false;
        };
        if arguments[1..].iter().any(|word| word.starts_with('-')) {
            return false;
        }
        // Single-quoted/escaped dollar signs are literal data, not caller input.
        if word.contains('\'') || word.contains('\\') {
            return false;
        }
        let operand = word
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(word);
        tokens.iter().any(|token| {
            operand == token
                || probe == "curl"
                    && operand
                        .strip_prefix("http://")
                        .or_else(|| operand.strip_prefix("https://"))
                        .is_some_and(|url| {
                            url.strip_prefix(token)
                                .is_some_and(|rest| rest.starts_with('/'))
                        })
        })
    })
}

#[test]
fn conditional_probe_rejects_multiline_literal_probe_text() {
    // Exact helper body from the reviewer's valid Bash printf-literal fixture.
    let helper = r#"
  local scope="$1"
  local target="$2"
  printf '%s\n' 'text
  if nc -z "$target"; then
    exit 1
  fi
  ' >/dev/null
  printf '{"scope":"%s","target":"%s","result":"denied"}\n' "$scope" "$target" >> "$evidence_file"
"#;
    assert!(!helper_has_caller_target_probe(helper));
}

#[test]
fn conditional_probe_rejects_quoted_heredoc_probe_text() {
    // Exact helper body from the reviewer's corrected, valid Bash heredoc fixture.
    let helper = r#"
  local scope="$1"
  local target="$2"
  cat <<'LITERAL' >/dev/null
  if nc -z "$target"; then
    exit 1
  fi
LITERAL
  printf '{"scope":"%s","target":"%s","result":"denied"}\n' "$scope" "$target" >> "$evidence_file"
"#;
    assert!(!helper_has_caller_target_probe(helper));
}

#[test]
fn conditional_probe_rejects_unsupported_helper_tokens_before_scanning() {
    let probe = "local target=\"$2\"\nif nc -z \"$target\"; then\nexit 1\nfi";
    for unsupported in [
        "printf 'unterminated",
        "printf \"unterminated",
        "printf trailing\\",
        "printf ${target",
        "printf $(echo data)",
        "printf `echo data`",
        "( printf data",
        "cat <<EOF",
        "cat <<<'data'",
        "unknown_command data",
        "fi",
        "else",
        "if nc -z \"$target\"; then",
    ] {
        // Validate even source after an otherwise acceptable probe: no early GREEN.
        let helper = format!("{probe}\n{unsupported}");
        assert!(!helper_has_caller_target_probe(&helper), "{unsupported}");
    }
}

#[test]
fn conditional_probe_rejects_comment_only_caller_target() {
    let helper = "local target=\"$2\"\nif nc -z 127.0.0.1 65535; then # \"$target\"\nexit 1\nfi";
    assert!(!helper_has_caller_target_probe(helper));
}

#[test]
fn conditional_probe_requires_target_in_probe_not_echo_or_inverted_success() {
    for condition in [
        "if nc -z 127.0.0.1 65535; echo \"$target\"; then",
        "if timeout 2 echo ' nc ' \"$target\"; then",
        "if nc -z '$target'; then",
        "if ! nc -z \"$target\"; then",
    ] {
        let helper = format!("local target=\"$2\"\n{condition}\nexit 1\nfi");
        assert!(!helper_has_caller_target_probe(&helper), "{condition}");
    }
}

#[test]
fn conditional_probe_rejects_target_as_non_endpoint_option_data() {
    for condition in [
        "if curl -d \"$target\" http://127.0.0.1/; then",
        "if nc 127.0.0.1 65535 -e \"$target\"; then",
        "if dig -f \"$target\"; then",
    ] {
        let helper = format!("local target=\"$2\"\n{condition}\nexit 1\nfi");
        assert!(!helper_has_caller_target_probe(&helper), "{condition}");
    }
}

#[test]
fn conditional_probe_preserves_caller_aliases_and_quoted_hash_inputs() {
    for condition in [
        "if nc -z \"$target\"; then # allowed comment",
        "if timeout 2 nc -z \"${target}\"; then",
        "if ping -c 1 \"$2\"; then",
        "if curl \"http://${2}/ # fragment\"; then # outside",
        "if nc -z \"$target\" >/dev/null 2>&1; then",
    ] {
        let helper = format!("local target=\"$2\"\n{condition}\nexit 1\nfi");
        assert!(helper_has_caller_target_probe(&helper), "{condition}");
    }
    assert_eq!(
        code_before_inline_comment(
            "probe_forbidden_endpoint \"DNS # scope\" \"host#value\" # outside"
        ),
        "probe_forbidden_endpoint \"DNS # scope\" \"host#value\""
    );
}

#[test]
fn caller_target_aliases_require_exact_positional_assignment() {
    let helper = "local target=\"$2\"\nendpoint=${2}\nignored='$2'\nfixed=127.0.0.1\n";
    let tokens = caller_target_tokens(helper);
    for expected in [
        "$2",
        "${2}",
        "$target",
        "${target}",
        "$endpoint",
        "${endpoint}",
    ] {
        assert!(tokens.contains(&expected.to_owned()));
    }
    for rejected in ["$ignored", "$fixed"] {
        assert!(!tokens.contains(&rejected.to_owned()));
    }
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

    assert!(
        helper_has_caller_target_probe(helper),
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
            direct_calls
                .iter()
                .any(|line| line.contains(required_scope)),
            "{required_scope} must be passed as executable helper input before any inline comment, not merely mentioned in output/configuration/comment text"
        );
    }
}
