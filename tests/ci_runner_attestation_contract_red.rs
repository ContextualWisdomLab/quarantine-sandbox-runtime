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

// Offline execution witness only. The gate is constrained before Bash starts;
// nc is a synthetic boundary and PATH never exposes a real network program.
fn script_has_observed_denial(script: &str) -> bool {
    script_has_observed_denial_with(script, execute_fixture)
}

fn script_has_observed_denial_with(
    script: &str,
    execute: impl Fn(&str, Option<i32>) -> FixtureRun,
) -> bool {
    if !supported_execution_script(script) {
        return false;
    }
    // Admission permits only quoted, whitespace-free call operands. Bind the
    // observations to those reviewed inputs, not merely to each other.
    let calls = script
        .lines()
        .filter_map(|line| simple_probe_words(line.trim()))
        .filter(|words| words.first() == Some(&"probe_forbidden_endpoint"))
        .map(|words| {
            words[1..]
                .iter()
                .map(|word| word.trim_matches('"'))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if calls.len() != 6 {
        return false;
    }
    for code in [Some(1), Some(0), Some(2), Some(124), Some(127), None] {
        let run = execute(script, code);
        if run.exit.is_none() {
            eprintln!("offline control TEST_NOT_COMPLETED for probe outcome {code:?}");
            return false;
        }
        if code != Some(1) {
            if run.exit == Some(0) || !run.evidence.trim().is_empty() {
                return false;
            }
        } else if run.exit != Some(0) || run.probes.len() != 6 {
            eprintln!(
                "offline denial control exit={:?} probes={} evidence={}",
                run.exit,
                run.probes.len(),
                run.evidence
            );
            return false;
        } else {
            let evidence: Result<Vec<serde_json::Value>, _> =
                run.evidence.lines().map(serde_json::from_str).collect();
            let Ok(evidence) = evidence else {
                return false;
            };
            let scopes = [
                "192.168.0.0/16",
                "10.0.0.0/8",
                "172.16.0.0/12",
                "6379",
                "5432",
                "DNS",
            ];
            if evidence.len() != scopes.len() {
                return false;
            }
            for (((row, probe), scope), call) in
                evidence.iter().zip(&run.probes).zip(scopes).zip(&calls)
            {
                if row.as_object().is_none_or(|object| object.len() != 6)
                    || row["scope"] != scope
                    || call[0] != scope
                    || row["target"] != call[1]
                    || row["command"] != "nc"
                    || row["protocol"] != "tcp"
                    || row["result"] != "denied"
                    || row["probe_exit"] != 1
                    || row["target"]
                        .as_str()
                        .is_none_or(|target| *probe != format!("-z {target}"))
                {
                    return false;
                }
            }
        }
    }
    true
}

fn supported_execution_script(script: &str) -> bool {
    let Some(start) = script.find("probe_forbidden_endpoint() {\n") else {
        return false;
    };
    let prefix = &script[..start];
    if prefix != "set -euo pipefail\nevidence_file=\"$PWD/evidence.json\"\n" {
        return false;
    }
    let helper_start = start + "probe_forbidden_endpoint() {\n".len();
    let Some(end) = script[helper_start..].find("\n}\n") else {
        return false;
    };
    let helper = &script[helper_start..helper_start + end];
    // Only these two bounded helper forms are executable. This is an adapter
    // contract, not a general shell parser; future probe adapters need review.
    let original = shell_function_body(ERROR_AS_DENIAL_SCRIPT, "probe_forbidden_endpoint").trim();
    let normalized = helper.replace(
        "  if [ \"$status\" -ne 1 ]; then",
        "  if nc -z \"$target\"; then",
    );
    if helper.trim() != original && !helper_has_supported_source(&normalized) {
        return false;
    }
    let mut lines = helper.lines();
    let expected = SAFE_HELPER.lines().collect::<Vec<_>>();
    if helper.trim() != original {
        for line in &expected[..expected.len() - 1] {
            if lines.next() != Some(*line) {
                return false;
            }
        }
        if lines.next() != expected.last().copied() || lines.next().is_some() {
            return false;
        }
    }
    let suffix = &script[helper_start + end + 3..];
    suffix.lines().all(|line| {
        let line = line.trim();
        if ["if false; then", "fi", ""].contains(&line) {
            return true;
        }
        simple_probe_words(line).is_some_and(|words| {
            words.len() == 3
                && words[0] == "probe_forbidden_endpoint"
                && words[1..].iter().all(|word| {
                    word.starts_with('"')
                        && word.ends_with('"')
                        && word[1..word.len() - 1]
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"./:-".contains(&b))
                })
        })
    })
}

struct FixtureRun {
    exit: Option<i32>,
    probes: Vec<String>,
    evidence: String,
}

fn execute_fixture(script: &str, probe_exit: Option<i32>) -> FixtureRun {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "qsr-offline-attestation-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).expect("fresh fixture directory");
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    if let Some(code) = probe_exit {
        let stub = bin.join("nc");
        fs::write(
            &stub,
            format!("#!/bin/bash\nprintf '%s\\n' \"$*\" >> \"$QSR_PROBE_LOG\"\nexit {code}\n"),
        )
        .unwrap();
        fs::set_permissions(stub, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(root.join("gate.sh"), script).unwrap();
    let stderr_file = fs::File::create(root.join("stderr.log")).unwrap();
    let mut child = Command::new("/bin/bash")
        .arg("gate.sh")
        .current_dir(&root)
        .env_clear()
        .env("PATH", &bin)
        .env("HOME", &root)
        .env("LC_ALL", "C")
        .env("QSR_PROBE_LOG", root.join("probes.log"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let exit = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code();
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            eprintln!(
                "offline fixture TEST_NOT_COMPLETED outcome={probe_exit:?}; stderr={}",
                fs::read_to_string(root.join("stderr.log")).unwrap_or_default()
            );
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let probes = fs::read_to_string(root.join("probes.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let evidence = fs::read_to_string(root.join("evidence.json")).unwrap_or_default();
    fs::remove_dir_all(root).unwrap();
    FixtureRun {
        exit,
        probes,
        evidence,
    }
}

const ERROR_AS_DENIAL_SCRIPT: &str = r#"set -euo pipefail
evidence_file="$PWD/evidence.json"
probe_forbidden_endpoint() {
  local scope="$1"
  local target="$2"
  if nc -z "$target"; then
    exit 1
  fi
  printf '{"scope":"%s","target":"%s","result":"denied"}\n' "$scope" "$target" >> "$evidence_file"
}
probe_forbidden_endpoint "192.168.0.0/16" "192.168.50.1"
probe_forbidden_endpoint "10.0.0.0/8" "10.23.4.5"
probe_forbidden_endpoint "172.16.0.0/12" "172.31.255.2"
probe_forbidden_endpoint "6379" "10.0.0.1:6379"
probe_forbidden_endpoint "5432" "192.168.50.1:5432"
probe_forbidden_endpoint "DNS" "169.254.1.53:53"
"#;

#[test]
fn executable_contract_rejects_dormant_helper_calls() {
    let dormant = ERROR_AS_DENIAL_SCRIPT.replacen(
        "probe_forbidden_endpoint \"192.168.0.0/16\"",
        "if false; then\nprobe_forbidden_endpoint \"192.168.0.0/16\"",
        1,
    ) + "fi\n";
    assert!(
        !script_has_observed_denial(&dormant),
        "zero executed probes cannot attest denial"
    );
}

const SAFE_HELPER: &str = r#"  local scope="$1"
  local target="$2"
  if nc -z "$target"; then
    exit 1
  else
    local status=$?
  fi
  if [ "$status" -ne 1 ]; then
    exit 2
  fi
  printf '{"scope":"%s","target":"%s","result":"denied","command":"nc","protocol":"tcp","probe_exit":1}\n' "$scope" "$target" >> "$evidence_file""#;

fn safe_fixture() -> String {
    let begin = ERROR_AS_DENIAL_SCRIPT.find("  local scope=").unwrap();
    let end = ERROR_AS_DENIAL_SCRIPT[begin..].find("\n}\n").unwrap() + begin;
    format!(
        "{}{}{}",
        &ERROR_AS_DENIAL_SCRIPT[..begin],
        SAFE_HELPER,
        &ERROR_AS_DENIAL_SCRIPT[end..]
    )
}

#[test]
fn executable_contract_rejects_unsupported_receipt_before_execution() {
    let receipt = SAFE_HELPER.lines().last().unwrap();
    for unsupported in [
        "  printf '%s\\n' \"$scope\" >> \"$evidence_file\"",
        "  printf -v receipt_buffer '%s' \"$scope\"",
    ] {
        let script = safe_fixture().replace(receipt, unsupported);
        assert!(
            !script_has_observed_denial_with(&script, |_, _| {
                panic!("unsupported receipt must be rejected before execution")
            }),
            "only the reviewed output-only receipt command may execute"
        );
    }
}

#[test]
fn executable_contract_accepts_observed_denial_control() {
    assert!(
        script_has_observed_denial(&safe_fixture()),
        "instrumented denial with explicit tool-error separation must pass"
    );
}

#[test]
fn executable_contract_rejects_unbound_evidence() {
    for (from, to) in [
        ("\"command\":\"nc\"", "\"command\":\"echo\""),
        ("\"protocol\":\"tcp\"", "\"protocol\":\"udp\""),
        ("\"probe_exit\":1", "\"probe_exit\":127"),
        ("\"result\":\"denied\"", "\"result\":\"connected\""),
    ] {
        assert!(
            !script_has_observed_denial(&safe_fixture().replace(from, to)),
            "{to}"
        );
    }
}

#[test]
fn executable_contract_rejects_incomplete_tool_error_control() {
    assert!(
        !script_has_observed_denial_with(&safe_fixture(), |script, code| {
            if code == Some(2) {
                FixtureRun {
                    exit: None,
                    probes: Vec::new(),
                    evidence: String::new(),
                }
            } else {
                execute_fixture(script, code)
            }
        }),
        "a timed-out error control is not completed rejection evidence"
    );
}

#[test]
fn executable_contract_rejects_observations_for_another_source_target() {
    let script = safe_fixture();
    let observed = execute_fixture(&script, Some(1));
    assert_eq!(observed.exit, Some(0));
    assert_eq!(observed.probes.len(), 6);
    let evidence = observed
        .evidence
        .lines()
        .map(|line| {
            let mut row: serde_json::Value = serde_json::from_str(line).unwrap();
            row["target"] = "127.0.0.1:9".into();
            row.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !script_has_observed_denial_with(&script, |_, code| {
            if code == Some(1) {
                FixtureRun {
                    exit: Some(0),
                    probes: vec!["-z 127.0.0.1:9".to_owned(); 6],
                    evidence: evidence.clone(),
                }
            } else {
                FixtureRun {
                    exit: Some(2),
                    probes: Vec::new(),
                    evidence: String::new(),
                }
            }
        }),
        "matching receipt/probe observations must still match the reviewed source targets"
    );
}

#[test]
fn executable_contract_validates_injected_evidence_after_source_admission() {
    let script = safe_fixture();
    let observed = execute_fixture(&script, Some(1));
    assert_eq!(observed.exit, Some(0));
    let rows = observed
        .evidence
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let render = |rows: &[serde_json::Value]| {
        rows.iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let accepts = |evidence: &str, probes: &[String]| {
        let calls = std::cell::Cell::new(0);
        let accepted = script_has_observed_denial_with(&script, |_, code| {
            calls.set(calls.get() + 1);
            if code == Some(1) {
                FixtureRun {
                    exit: Some(0),
                    probes: probes.to_owned(),
                    evidence: evidence.to_owned(),
                }
            } else {
                FixtureRun {
                    exit: Some(2),
                    probes: Vec::new(),
                    evidence: String::new(),
                }
            }
        });
        assert!(
            calls.get() > 0,
            "source admission must reach the observations"
        );
        if accepted {
            assert_eq!(calls.get(), 6, "all outcome controls must complete");
        }
        accepted
    };
    assert!(accepts(&render(&rows), &observed.probes));
    for malformed in ["", "{", "[]", "null"] {
        assert!(!accepts(malformed, &observed.probes), "{malformed}");
    }
    assert!(!accepts(&render(&rows[..5]), &observed.probes));
    let mut extra = rows.clone();
    extra.push(rows[0].clone());
    assert!(!accepts(&render(&extra), &observed.probes));
    for (field, value) in [
        ("scope", serde_json::json!("DNS")),
        ("command", serde_json::json!("echo")),
        ("protocol", serde_json::json!("udp")),
        ("target", serde_json::json!("127.0.0.1:9")),
        ("target", serde_json::Value::Null),
        ("result", serde_json::json!("connected")),
        ("probe_exit", serde_json::json!(127)),
        ("probe_exit", serde_json::json!("1")),
        ("unknown", serde_json::json!(true)),
    ] {
        let mut mutated = rows.clone();
        mutated[0][field] = value;
        assert!(!accepts(&render(&mutated), &observed.probes), "{field}");
    }
    let mut missing = rows.clone();
    missing[0].as_object_mut().unwrap().remove("command");
    assert!(!accepts(&render(&missing), &observed.probes));
    assert!(!accepts(&render(&rows), &observed.probes[..5]));
    let mut mismatched = observed.probes.clone();
    mismatched[0] = "-z 127.0.0.1:9".to_owned();
    assert!(!accepts(&render(&rows), &mismatched));
}

#[test]
fn executable_contract_preserves_admitted_call_whitespace() {
    for (command_separator, operand_separator) in
        [(" ", " "), ("\t", " "), (" \t ", "\t"), ("\t ", " \t ")]
    {
        let script = safe_fixture()
            .lines()
            .map(|line| {
                if line.starts_with("probe_forbidden_endpoint ") {
                    line.replace(
                        "probe_forbidden_endpoint \"",
                        &format!("probe_forbidden_endpoint{command_separator}\""),
                    )
                    .replace("\" \"", &format!("\"{operand_separator}\""))
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(supported_execution_script(&script));
        assert!(
            script_has_observed_denial(&script),
            "admitted whitespace must keep the same observed-call contract: {command_separator:?}/{operand_separator:?}"
        );
    }
}

#[test]
fn executable_contract_rejects_source_call_count_before_execution() {
    let script = safe_fixture();
    let first_call = "probe_forbidden_endpoint \"192.168.0.0/16\" \"192.168.50.1\"\n";
    let no_calls = script
        .lines()
        .filter(|line| !line.starts_with("probe_forbidden_endpoint \""))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    for candidate in [
        script.replacen(first_call, "", 1),
        format!("{script}{first_call}"),
        no_calls,
    ] {
        assert!(supported_execution_script(&candidate));
        assert!(!script_has_observed_denial_with(&candidate, |_, _| {
            panic!("zero, missing or extra calls must reject before execution")
        }));
    }
}

#[test]
fn executable_contract_requires_ordered_unique_source_scopes() {
    let script = safe_fixture();
    let calls = script
        .lines()
        .filter(|line| line.starts_with("probe_forbidden_endpoint \""))
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 6);
    let prefix = script.split_once(calls[0]).unwrap().0;
    let mut candidates = vec![(calls.clone(), true)];
    for index in 0..calls.len() - 1 {
        let mut swapped = calls.clone();
        swapped.swap(index, index + 1);
        candidates.push((swapped, false));
    }
    for index in 1..calls.len() {
        let mut duplicate = calls.clone();
        duplicate[index] = calls[0];
        candidates.push((duplicate, false));
    }
    for (source_calls, expected) in candidates {
        let candidate = format!("{prefix}{}\n", source_calls.join("\n"));
        assert!(supported_execution_script(&candidate));
        // Matching synthetic source/probe/receipt records are insufficient when
        // the required six scopes are reordered or duplicated. No Bash runs.
        let rows = source_calls
            .iter()
            .map(|call| {
                let words = simple_probe_words(call).unwrap();
                serde_json::json!({
                    "scope": words[1].trim_matches('"'),
                    "target": words[2].trim_matches('"'),
                    "command": "nc",
                    "protocol": "tcp",
                    "result": "denied",
                    "probe_exit": 1
                })
            })
            .collect::<Vec<_>>();
        let probes = rows
            .iter()
            .map(|row| format!("-z {}", row["target"].as_str().unwrap()))
            .collect::<Vec<_>>();
        let evidence = rows
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        let invocations = std::cell::Cell::new(0);
        let accepted = script_has_observed_denial_with(&candidate, |_, outcome| {
            invocations.set(invocations.get() + 1);
            if outcome == Some(1) {
                FixtureRun {
                    exit: Some(0),
                    probes: probes.clone(),
                    evidence: evidence.clone(),
                }
            } else {
                FixtureRun {
                    exit: Some(2),
                    probes: Vec::new(),
                    evidence: String::new(),
                }
            }
        });
        assert_eq!(accepted, expected, "{source_calls:?}");
        assert_eq!(invocations.get(), if expected { 6 } else { 1 });
    }
}

#[test]
fn executable_contract_rejects_completed_non_denial_success_or_evidence() {
    let script = safe_fixture();
    assert!(supported_execution_script(&script));
    let calls = script
        .lines()
        .filter_map(simple_probe_words)
        .filter(|words| words.first() == Some(&"probe_forbidden_endpoint"))
        .collect::<Vec<_>>();
    let rows = calls
        .iter()
        .map(|words| {
            serde_json::json!({
                "scope": words[1].trim_matches('"'),
                "target": words[2].trim_matches('"'),
                "command": "nc",
                "protocol": "tcp",
                "result": "denied",
                "probe_exit": 1
            })
        })
        .collect::<Vec<_>>();
    let probes = rows
        .iter()
        .map(|row| format!("-z {}", row["target"].as_str().unwrap()))
        .collect::<Vec<_>>();
    let evidence = rows
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    let outcomes = [Some(1), Some(0), Some(2), Some(124), Some(127), None];
    // Inject observations only: no subprocess or network program runs here.
    // The valid denial control must reach each completed non-denial control.
    for rejected_outcome in outcomes[1..].iter().copied() {
        for fault in [None, Some((0, "")), Some((2, "stray receipt"))] {
            let observed = std::cell::RefCell::new(Vec::new());
            let accepted = script_has_observed_denial_with(&script, |_, outcome| {
                observed.borrow_mut().push(outcome);
                if outcome == Some(1) {
                    FixtureRun {
                        exit: Some(0),
                        probes: probes.clone(),
                        evidence: evidence.clone(),
                    }
                } else {
                    let (exit, evidence) = if outcome == rejected_outcome {
                        fault.unwrap_or((2, " \t\n"))
                    } else {
                        (2, " \t\n")
                    };
                    FixtureRun {
                        exit: Some(exit),
                        probes: Vec::new(),
                        evidence: evidence.to_owned(),
                    }
                }
            });
            assert_eq!(accepted, fault.is_none(), "{rejected_outcome:?}/{fault:?}");
            let count = if fault.is_none() {
                outcomes.len()
            } else {
                outcomes
                    .iter()
                    .position(|code| *code == rejected_outcome)
                    .unwrap()
                    + 1
            };
            assert_eq!(*observed.borrow(), outcomes[..count]);
        }
    }
}

#[test]
fn executable_contract_rejects_tool_errors_as_denial() {
    assert!(
        !script_has_observed_denial(ERROR_AS_DENIAL_SCRIPT),
        "probe errors must not become denial receipts"
    );
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
    let executable = script
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(
        script_has_observed_denial(&executable),
        "offline execution must bind six actual probes and JSON evidence, and reject reachable/tool-error/missing-tool outcomes"
    );
}
