# Single Inspection Cardinality Traceability

## Decision

Podman inspection evidence that is modeled as a single runtime record is admitted only when the decoded JSON array contains exactly one element. Zero or multiple records remain `MalformedIsolationInspection`; after the `len() == 1` guard succeeds, extracting the sole element is an internal invariant rather than another fallible business/security outcome.

## Causal evidence

Exact #112 predecessor `61d6ca17cc202ac264fc3b1796b649001652e505` completed repository policy, rustfmt, the full workspace/all-target/no-fail-fast test suite, Clippy `-D warnings`, rustdoc `-D warnings`, and hosted negative rootless/AppArmor verification. Its immutable branch artifact (`sha256:d347e4665d940501101eccc7041183fe814249a79001818d37969e91396b8bec`) still reported an uncovered `None` arm after `values.len() == 1` in `parse_single_inspection`.

That arm cannot occur: for a `Vec<T>` whose length has just been proven to be exactly one, removing index zero is defined. Treating a second `Option` failure as a production outcome inflated the owned branch surface without representing a hostile input, runtime failure, or recoverable state.

One-shot repair run `34626358364` replaced only that redundant `pop().ok_or(...)` branch with `Ok(values.remove(0))`. Before publishing its ordinary descendant it ran the focused runtime-evidence-cardinality regression, full workspace tests, rustfmt, Clippy, rustdoc, and `git diff --check`; all passed. The temporary source-fix workflow removed itself in the same descendant.

## Security semantics retained

The externally meaningful cardinality guard is unchanged: decoded arrays with `len() != 1` still fail closed as `MalformedIsolationInspection`. Container identity matching, configured capability checks, PID-1 process evidence cardinality, effective seccomp/LSM checks, cleanup ownership, and exact-ID lifecycle authority are not relaxed.

No coverage ignore/exclusion, denominator configuration, parser tolerance widening, or error-class weakening is used. The change removes only a control-flow outcome contradicted by the immediately preceding invariant.

## Acceptance and rollback

The repair is acceptable only after the current exact #112 descendant reacquires native CI on its own SHA. Repository-wide 100% line/function/region/branch coverage, positive effective-LSM, issues #35/#43 real-runtime acceptance, independent review/security, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates.

Rollback is an ordinary revert of the structural refactor if a future parser design no longer proves exactly-one cardinality before extraction. In that case the newly reachable failure must be represented by a typed contract and a causal regression rather than reintroduced solely as uncovered defensive control flow.
