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

## 2026-09-15 review repair: lifetimes, labels, and Supporting-context submodules

Current-head review exposed two additional fitness-rule gaps that are distinct from raw strings.

First, Rust lifetime and loop-label syntax uses a leading single quote. The dependency regex treated `'application_service` as the identifier `application_service`, so a Core function could be rejected merely for using a legal lifetime or label with that name. Checked-in regression `990c4b8d356a9aefadd25b581988c209cdd035dd` adds both lifetime and loop-label witnesses. A deterministic local evaluation of the prior regex reproduced both false positives while a real `crate::application_service::ApplicationServiceRequest` reference remained positive. Commit `13467248e955853c050248a6eb2bf55f8dc717c1` narrows the matcher only for Supporting-context tokens immediately preceded by a single quote. It does not suppress ordinary module paths or `ApplicationService*` type names.

Second, the Rust architectural fitness test previously inspected only `src/application_service/mod.rs`. That left a structural loophole: a Supporting-context submodule could import a Core-private `CommandExecutionOutcome`, and the parent module could expose it indirectly. Commit `0b6c6f72ea12c2f1a25faebff3c0135646a476fa` changes the test to recursively inspect every Rust source under `src/application_service/`. It rejects any `CommandExecutionOutcome` occurrence in that bounded context and also rejects a whitespace-normalized `sandbox_execution::*` wildcard import. The existing infrastructure rule requiring bounded-command domain truth to come directly from `sandbox_execution` remains in place.

## 2026-09-16 review repair: grouped and nested glob use trees

Review of exact `82c15a5d20c2eab710f59785e291836c48b2d176` found that the Supporting-context wildcard fitness rule recognized only the direct normalized form `sandbox_execution::*`. Rust also permits brace-grouped use trees, including `use crate::sandbox_execution::{*};`, `use crate::sandbox_execution::{CommandExecutionRequest, *};`, and nested forms such as `use crate::sandbox_execution::{nested::{Thing}, *};`. Those forms could therefore bypass the architecture test while importing the full Core export surface.

Commit `e5a2be3ca26245f3d48295fa72e78d716a7c84ae` replaced the single substring check with the first `imports_sandbox_execution_glob` use-tree detector and added regressions for direct, grouped, mixed named-plus-glob, and root-grouped forms. Named imports remained allowed. This closed the root-grouped loophole but, as the next exact-head review demonstrated, the helper still assumed the tail immediately after `sandbox_execution::` began with either `*` or `{` and therefore did not yet cover a descendant path before the glob leaf.

The Rust Reference defines `UseTree` recursively: a use tree may be `*`, a brace-delimited list of use trees, or a named path, and brace groups may nest. It explicitly gives a nested glob example equivalent in shape to `use a::b::{self as ab, c, d::{*, e::f}};`. The fitness rule therefore has to recognize wildcard leaves in the whole use tree rather than only the direct `path::*` spelling.

### Descendant-path glob follow-up

Fresh review of exact `363a822a8f8e2b967f418d28e5992b24121264f5` found the remaining structural hole. The helper rejected `sandbox_execution::*` and root brace groups, but returned false for legal descendant-path forms such as `use crate::sandbox_execution::nested::*;` because the tail begins with `nested::`. The same invariant must also cover brace-grouped descendant leaves such as `use crate::sandbox_execution::{nested::*};` and deeper groups such as `use crate::sandbox_execution::nested::{Thing, *};`.

Checked-in regression `be6e8f0927f5e5e43a1abad51787618174536690` adds direct descendant, root-grouped descendant, and descendant brace-group glob witnesses, plus named descendant imports as negative controls. This is source RED derived from a verified exact-head review finding; no hosted execution is claimed for that intermediate head.

Commit `c64353cebbc5f3e479947415a08c383e266aa7b3` changes the focused detector to evaluate the complete `sandbox_execution::` use-tree segment through its terminating semicolon and reject any `*` leaf in that segment. Because `*` has glob meaning inside a Rust `UseTree`, this covers direct, grouped, mixed and arbitrarily descendant glob leaves while leaving named descendant imports accepted. Production runtime Rust, public contracts and domain ownership are unchanged.

### Non-code wildcard lexical follow-up

Fresh review of exact `4cfbebf8c9d511d3ac1b07ffeb825be0a334a19c` found that the descendant-glob detector still compacted the entire source before searching for `sandbox_execution::` and `*`. A comment, diagnostic string, or raw string containing example text such as `use crate::sandbox_execution::*;` was therefore indistinguishable from an executable `use` declaration and could reject a valid Supporting-context source file.

Checked-in regression `826e7d918bd8ee5499240e819c915d9865f54d43` adds line-comment, ordinary-string, raw-string, and nested-block-comment negative controls. The pre-repair helper classifies those examples as wildcard imports because whitespace compaction preserves their path and `*` tokens. Native CI `35011914210` materialized for that exact head but all five jobs were cancelled with `runner_id=0`, `steps=[]`; this is checked-in deterministic RED source, not hosted executed causal RED.

Commit `3c379187bcd03768819447ebd7253f1a9c569939` makes the Rust architecture fitness helper apply the same lexical boundary already used by `scripts/validate_repository.py`: nested block comments, line comments, ordinary quoted strings, and `r`/`br`/`cr` raw strings are masked before use-tree inspection, while newlines are preserved. The wildcard detector then operates only on remaining Rust code. Direct, grouped, descendant, and nested executable glob controls stay positive. This is a test/fitness-rule repair only; production runtime Rust, public APIs, schemas, isolation semantics, and coverage thresholds are unchanged.

Rust treats non-doc comments as whitespace and permits nested block comments. The `UseTree` grammar assigns glob semantics to `*` only inside a `use` declaration. Treating comment/literal payload as import syntax therefore creates a policy false positive rather than additional DDD enforcement.

These changes are repository-policy and architecture-test repairs only. Hosted exact-head evidence remains required after every descendant commit; no predecessor GREEN transfers.

## Alternatives rejected

- Renaming documentation, lifetimes, labels, or diagnostic strings: hides validator defects and leaves equivalent legal Rust syntax vulnerable.
- Whitelisting specific messages or files: converts a structural rule into mutable exceptions.
- Removing the `ApplicationService*` / `application_service` fitness rule: weakens the DDD boundary rather than fixing lexical classification.
- Scanning only the Supporting-context root module: does not protect the bounded context when implementation is split across submodules.
- Checking only `sandbox_execution::*` and `sandbox_execution::{*}` as literal strings: misses mixed, descendant-path and nested brace use trees that Rust permits.
- Stopping use-tree inspection after the first descendant identifier: misses legal `path::*` and `path::{..., *}` forms below the Core context root.
- Adding a full Rust parser solely for this rule: materially increases repository-policy dependency and maintenance surface for a bounded lexical requirement. Revisit only if future Rust token classes or use-tree forms make the local detector materially incomplete.

## Verification and remaining gate

The repair candidate must pass the focused architecture regression and the normal exact-head repository policy, rustfmt, full workspace tests, Clippy with warnings denied, public/private rustdoc with warnings denied, owned-production coverage, security/review gates, and applicable real isolation evidence. No predecessor GREEN transfers after the head moves.

## References

Rust Project Developers. (n.d.). *Tokens*. The Rust Reference. Retrieved September 15, 2026, from https://doc.rust-lang.org/reference/tokens.html

Rust Project Developers. (n.d.). *Use declarations*. The Rust Reference. Retrieved September 16, 2026, from https://doc.rust-lang.org/reference/items/use-declarations.html

Rust Project Developers. (n.d.). *Comments*. The Rust Reference. Retrieved September 16, 2026, from https://doc.rust-lang.org/reference/comments.html

The Rust Reference defines raw string literals as `r` followed by fewer than 256 `#` characters and a quote, terminated only by a quote followed by the same number of `#` characters. It separately defines raw byte (`br`) and raw C (`cr`, Edition 2021+) string literal token forms. Embedded quotes and backslashes that do not form the matching closing delimiter have no special terminating meaning inside a raw literal. Rust lifetime and loop-label tokens use a leading single quote followed by an identifier; that quote is syntax, not a Supporting-context path separator. The use-declaration grammar defines recursive brace-grouped use trees and glob leaves, including descendant-path and nested glob imports; non-doc comments are whitespace and nested block comments are supported.
