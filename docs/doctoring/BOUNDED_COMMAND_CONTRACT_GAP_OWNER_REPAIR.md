# Bounded command contract gap-owner repair

## Historical contract authority

PR #13 remains an open historical prerequisite for the provider-neutral bounded command-execution contract. Its valid delta includes ADR-0007, request/result schemas, validation and consumer-contract semantics, plus the rule that nonzero workload exit status is structured workload evidence while runtime/administrative failures remain typed errors.

Current DDD authority no longer places this domain in Supporting `application_service`. Issue #116 and canonical descendant #112 move bounded command execution into Core `sandbox_execution`. This repair must therefore preserve #13 provenance without reviving the obsolete placement.

## Successor boundary

A successor existing is not enough to close #13. #112 must completely retain every valid #13 delta/test/fixture/contract/evidence and satisfy its remaining private cross-context reach-through repair, exact-head tests/rustdoc/coverage, applicable owner integration, protected integration, and immutable release gates before #13 can reach PR zero by verified succession.

## Single-writer and parent-adoption decision

Review `5230246161` found that #13 still carried repository-wide `docs/product-technical-gap-baseline.md` and trailed its release-evidence parent. The repository-wide ledger remains #121 authority. #13-specific command-contract provenance and the Core-ownership supersession are preserved here before removing the leaf-owned Gap delta.

The branch then adopts current #10 exact `61e3b0489bdfd799e1b0d2269ca3f2c48b466102` through ordinary non-force ancestry, inheriting the parent owner-local doctoring and restored global-Gap state while retaining #13's historical contract files. No force push, destructive rebase, command-domain source resurrection, or predecessor-evidence transfer is permitted.

The resulting exact head remains Draft/open historical prerequisite authority. ADR-0007 stays Proposed until the canonical Core-owned successor completes its integration and release gates.
