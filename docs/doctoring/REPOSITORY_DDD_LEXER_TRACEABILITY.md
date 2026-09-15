# Repository DDD Lexer Traceability

## Problem

The repository fitness rule scans Rust source under `src/sandbox_execution/` for executable references to the Supporting `application_service` context. The earlier scanner removed ordinary comments and ordinary string contents before applying the dependency regex, but it did not understand Rust raw-string delimiters. A valid raw string such as `r#"compatibility " ApplicationServiceError text"#` therefore exposed text after the embedded quote to the dependency matcher and could reject documentation, diagnostic, or test data as if it were a Core-to-Supporting compile-time dependency.

This is a repository-policy false positive. The DDD rule itself remains valid: executable references such as `crate::application_service` and `ApplicationService*` types are still forbidden in the Core bounded context.

## Constraints

- Preserve the existing Core → Supporting dependency prohibition.
- Ignore only non-executable Rust literal/comment contents; do not add path- or message-specific suppression.
- Keep nested block-comment handling intact.
- Recognize Rust raw string, raw byte string, and raw C string delimiters without introducing a Rust compiler/parser dependency into the repository validator.
- Preserve source newlines while masking literal content so diagnostics remain stable.
- Do not change runtime Rust, public APIs, schemas, isolation semantics, coverage thresholds, or release gates.

## RED and causal evidence

Exact test-only `076a07a74603d35a9197d590556ceb826003440a` added `test_raw_string_literal_with_embedded_quote_does_not_create_false_dependency`. The checked-in witness demonstrates the lexical defect: the previous helper treated the embedded `"` as the end of an ordinary string and subsequently exposed `ApplicationServiceError` to `SUPPORTING_CONTEXT_REFERENCE`.

The hosted Actions run for that exact head did not execute before the repair lane advanced, so `076a07a...` is retained as checked-in RED source rather than claimed as hosted causal RED. Deterministic evaluation of the helper reproduced the false positive while the real `use crate::application_service::ApplicationServiceRequest;` controls remained positive.

## Selected repair

Commit `fa6886cb27cde0b7c6da07aff45bcffaa65ef5dd` adds `rust_raw_string_end`, recognizing the Rust raw literal prefixes `r`, `br`, and `cr`, the opening quote, and the exact closing quote-plus-hash delimiter. Raw literal content is masked before ordinary comment/string handling. An unmatched raw delimiter consumes the remaining source as non-executable text; syntactically invalid Rust is still rejected later by the Rust toolchain.

Test-only descendant `4ecb5894d51df40334eb78e2e2e7d8cc5ee9b7f3` adds multiple-hash, raw-byte, and post-raw-string real-dependency controls. The repair does not weaken the executable dependency matcher.

## Alternatives rejected

- Renaming documentation or diagnostic strings: hides the validator defect and leaves equivalent raw strings vulnerable.
- Whitelisting specific messages or files: converts a structural rule into mutable exceptions.
- Removing the `ApplicationService*` / `application_service` fitness rule: weakens the DDD boundary rather than fixing lexical classification.
- Adding a full Rust parser solely for this rule: materially increases repository-policy dependency and maintenance surface for a bounded lexical requirement. Revisit only if future Rust token classes make the local lexer materially incomplete.

## Verification and remaining gate

The repair candidate must pass the focused repository-validator regressions and the normal exact-head repository policy, rustfmt, full workspace tests, Clippy with warnings denied, public/private rustdoc with warnings denied, owned-production coverage, security/review gates, and applicable real isolation evidence. No predecessor GREEN transfers after the head moves.

## Reference

Rust Project Developers. (n.d.). *Tokens*. The Rust Reference. Retrieved September 15, 2026, from https://doc.rust-lang.org/reference/tokens.html

The Rust Reference defines raw string literals as `r` followed by fewer than 256 `#` characters and a quote, terminated only by a quote followed by the same number of `#` characters. It separately defines raw byte (`br`) and raw C (`cr`, Edition 2021+) string literal token forms. Embedded quotes and backslashes that do not form the matching closing delimiter have no special terminating meaning inside a raw literal.
