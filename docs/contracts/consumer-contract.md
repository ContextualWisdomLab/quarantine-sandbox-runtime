# Published consumer contract

This file is the published contract a host consumes. Pin a revision of
`ContextualWisdomLab/quarantine-sandbox-runtime` and read this file from
**this** repository. Do not require a sibling checkout of naruon, wardnet,
gyeot, or EgressWeave. Do not copy unpublished files from those trees.

Naruon and gyeot are allowed composition hubs. They call this contract.
This leaf is not folded into them.

## Product surface on the default branch

The default branch publishes this narrative contract and the ADRs. It
does not publish an executable package, OpenAPI document, or test
harness. When a runtime implementation is published from this same
repository, versioned machine-readable schemas will be added beside this
file. Until then, hosts bind to the rules below, not to an invented API.

## Caller responsibilities

The host:

1. Already holds the artifact bytes (collection, acquisition, and
   preservation stay with the host).
2. Submits those bytes plus only the bounded source context it is willing
   to disclose (channel class, filename the host already knows, and
   similar non-secret metadata).
3. Does not send source-system credentials, session cookies, or
   installation tokens to this leaf.
4. Interprets the response as analysis evidence, not as a malicious or
   benign verdict.
5. Keeps admission, WAF/IDS enforcement, outbound HTTP, notification,
   and retention in the host that owns those authorities.

## Leaf responsibilities

This leaf:

1. Treats the submitted bytes as the artifact identity.
2. Remains source-agnostic: the same contract applies regardless of
   whether the host is naruon, wardnet, gyeot, or an independent
   operator.
3. Remains credential-free: it does not fetch the source system.
4. Returns analysis evidence when a runtime exists, or publishes only
   this contract while the default branch is documentation-only.
5. Does not claim WAF/IDS, email admission, or outbound HTTP authority.

## Integration path

```text
host (naruon | gyeot | wardnet | independent operator)
  -> pin ContextualWisdomLab/quarantine-sandbox-runtime
  -> read docs/contracts/consumer-contract.md
  -> submit already-held artifact bytes + bounded context
  -> receive analysis evidence (when a runtime is published)
  -> apply host-owned policy
```

A hub wires the call in the hub’s own repository. This leaf does not
import hub source.

## Related decisions

- [ADR 0001](../adr/0001-product-authority-boundary.md)
- [ADR 0002](../adr/0002-credential-free-source-agnostic-analysis.md)
- [ADR 0003](../adr/0003-published-contract-consumption.md)
- [ADR 0004](../adr/0004-omit-unimplemented-scope.md)
