# Published consumer contract

This file is the published contract a host consumes. A host MUST pin an
immutable revision of `ContextualWisdomLab/quarantine-sandbox-runtime` and
read this file from **this** repository. An immutable revision is either:

- a full Git commit SHA; or
- a released package or container artifact pinned by cryptographic content
  digest, with its provenance or attestation verified.

A branch name, release tag, or package version label by itself is movable or
re-publishable and is not a sufficient pin. In this contract, `revision`
means one of the immutable identifiers above.

Do not require a sibling checkout of naruon, wardnet, gyeot, or EgressWeave.
Do not copy unpublished files from those trees.

Naruon and gyeot are allowed composition hubs. They call this contract.
This leaf is not folded into them.

## Product surface on the default branch

The default branch publishes this narrative contract and the ADRs. It
does not publish an executable package, OpenAPI document, or test
harness. When a runtime implementation is published from this same
repository, versioned machine-readable schemas will be added beside this
file. Until then, hosts bind to the rules below, not to an invented API.

## Normative bounded source context

`bounded_source_context` is optional, untrusted metadata supplied by the host.
It is not part of artifact identity. The only permitted fields are:

| Field | Requirement |
| --- | --- |
| `source_channel_code` | ASCII enum value no longer than 64 bytes, such as `email_attachment`, `issue_upload`, `connector_input`, or `direct_api`. |
| `original_file_name` | UTF-8 basename no longer than 255 bytes. It MUST NOT contain a directory path. If the name contains personal or secret data that the host cannot safely disclose, the host MUST omit it. |
| `declared_media_type` | ASCII media-type hint no longer than 127 bytes. It is untrusted and MUST NOT override byte-derived analysis. |
| `host_artifact_reference` | Opaque ASCII reference no longer than 128 bytes. It MUST NOT encode an email address, URL, credential, token, or other personal identifier. |
| `submitted_at` | RFC 3339 UTC timestamp no longer than 35 bytes. |

The UTF-8 serialization of the complete context MUST NOT exceed 1,024 bytes.
Unknown fields, over-limit values, invalid encodings, and non-conforming values
MUST be rejected rather than truncated or silently accepted. Free-form notes,
arbitrary headers, and nested objects are not allowed.

The context MUST NOT contain passwords, API keys, personal access tokens,
OAuth authorization codes, session cookies, `Authorization` values, private
keys, installation tokens, database connection strings, signed URLs, URL query
strings, message bodies, postal addresses, phone numbers, government
identifiers, payment-card data, health data, or other source-system secrets or
unnecessary personal data. Artifact bytes may themselves contain sensitive
business content; this rule minimizes duplicated source metadata and does not
alter or mask the submitted artifact.

A future runtime MUST exclude bounded source context from general logs,
metrics, traces, crash reports, and model prompts. It may hold the context only
for the active request and MUST discard transient copies when the request
completes or aborts. Host-owned retention and legal preservation remain in the
host.

## Caller responsibilities

The host:

1. Already holds the artifact bytes (collection, acquisition, and
   preservation stay with the host).
2. Submits those bytes and, only when needed, `bounded_source_context` that
   conforms exactly to the normative definition above.
3. Does not send source-system credentials, session cookies, installation
   tokens, prohibited personal data, or unknown metadata fields to this leaf.
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
4. Validates optional bounded source context against the closed allowlist and
   deletion rules above.
5. Returns analysis evidence when a runtime exists, or publishes only
   this contract while the default branch is documentation-only.
6. Does not claim WAF/IDS, email admission, or outbound HTTP authority.

## Integration path

```text
host (naruon | gyeot | wardnet | independent operator)
  -> pin full commit SHA or released artifact digest
  -> read docs/contracts/consumer-contract.md
  -> submit already-held artifact bytes + optional bounded_source_context
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
