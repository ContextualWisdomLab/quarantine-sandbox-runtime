# Single Inspection Cardinality Traceability

## Decision

Podman inspection evidence that is modeled as a single runtime record is admitted only when the decoded JSON array contains exactly one element. Zero or multiple records remain `MalformedIsolationInspection`; after the `len() == 1` guard succeeds, extracting the sole element is an internal invariant rather than another fallible business/security outcome.

## Causal evidence

Exact #112 predecessor `61d6ca17cc202ac264fc3b1796b649001652e505` completed repository policy, rustfmt, the full workspace/all-target/no-fail-fast test suite, Clippy `-D warnings`, rustdoc `-D warnings`, and hosted negative rootless/AppArmor verification. Its immutable branch artifact (`sha256:d347e4665d940501101eccc7041183fe814249a79001818d37969e91396b8bec`) still reported an uncovered `None` arm after `values.len() == 1` in `parse_single_inspection`.

That arm cannot occur: for a `Vec<T>` whose length has just been proven to be exactly one, removing index zero is defined. Treating a second `Option` failure as a production outcome inflated the owned branch surface without representing a hostile input, runtime failure, or recoverable state.

One-shot repair run `34626358364` replaced only that redundant `pop().ok_or(...)` branch with `Ok(values.remove(0))`. Before publishing its ordinary descendant it ran the focused runtime-evidence-cardinality regression, full workspace tests, rustfmt, Clippy, rustdoc, and `git diff --check`; all passed. The temporary source-fix workflow removed itself in the same descendant.

The first bot-authored repair descendant produced an `action_required` pull-request workflow with zero jobs, so that run is not treated as exact-head GREEN. Connector-authored exact `2d5bab57e7d8bd2d9c0ee733171a8ac19a355d51` then reacquired native CI: verify `103353148163` and hosted negative rootless/AppArmor `103353148280` are GREEN, while coverage artifacts fail only the explicit repository-wide 100% admissions. Its exact evidence is 4787/4905 lines (97.59%), 442/453 functions (97.57%), 6388/6602 regions (96.76%), and 694/720 branches (96.39%); coverage digest `sha256:f7bb47288868a7413e71a147606b2d11f6d0eb7a8d21f2522fa9d0d0c8ad64de`, branch digest `sha256:620ddfc6a748b309c2e62979c5d456f38de886ba8eba0b46ef10f2b39cf2be83`.

The same authority has been prepended to `docs/product-technical-gap-baseline.md` without deleting prior causal history. That baseline update was performed by a purpose-limited workflow that removed itself after verifying the inserted exact SHA, evidence digests, and a clean diff. Because that workflow's publication commit is bot-authored, this traceability update intentionally creates a normal descendant that must reacquire native exact-head CI rather than transferring predecessor GREEN.

## Security semantics retained

The externally meaningful cardinality guard is unchanged: decoded arrays with `len() != 1` still fail closed as `MalformedIsolationInspection`. Container identity matching, configured capability checks, PID-1 process evidence cardinality, effective seccomp/LSM checks, cleanup ownership, and exact-ID lifecycle authority are not relaxed.

No coverage ignore/exclusion, denominator configuration, parser tolerance widening, or error-class weakening is used. The change removes only a control-flow outcome contradicted by the immediately preceding invariant.

## Acceptance and rollback

Repository-wide 100% line/function/region/branch coverage, positive effective-LSM, issues #35/#43 real-runtime acceptance, independent review/security, protected integration, and immutable release/SBOM/provenance/reproducibility/rollback remain separate gates. A documentation-only descendant is not merge authority until its own native exact-head checks complete.

Rollback is an ordinary revert of the structural refactor if a future parser design no longer proves exactly-one cardinality before extraction. In that case the newly reachable failure must be represented by a typed contract and a causal regression rather than reintroduced solely as uncovered defensive control flow.
