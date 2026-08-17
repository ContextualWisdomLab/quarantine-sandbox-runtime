# Technical Requirements Document

## Runtime contract

```text
AnalysisRequest + artifact name + bytes
                |
                v
        bounded ingestion
                |
                v
  SHA-256 + artifact classification
                |
                v
       ordered static analyzers
                |
                v
 evidence normalization + runtime manifest
                |
                v
          EvidenceBundle
```

## Rust modules

- `contracts`: versioned request, descriptor, evidence, manifest, and validation types.
- `ingestion`: bounded byte intake, immutable hash, and deterministic format detection.
- `runtime`: analyzer port, analyzer execution, evidence ordering, limitations, and serialization.

## Determinism

The runtime does not include wall-clock timestamps in the foundation output. A job identifier is derived from request ID, requested profile, artifact SHA-256, policy ID, runtime source revision, and the ordered analyzer identifiers. Evidence identifiers derive from the job identifier and one-based sequence number. Ordered maps are used for user and analyzer attributes.

This prevents two analyses performed under different policies, revisions, or analyzer portfolios from claiming the same evidence identity. A production host must construct the engine with an exact build revision rather than relying on the development default.

## Analyzer authority

Analyzer identifiers must be unique, bounded, and free of control characters. Static analyzers may emit only `file_format` or `static_capability` findings. The runtime owns `artifact_identity` and `policy_boundary`; future isolated workers own runtime and network observations. A static analyzer that claims another evidence category is converted to attributable `tool_failure` evidence and makes the result `inconclusive`.

## Validation bounds

| Field | Limit |
|---|---:|
| request ID | 128 UTF-8 bytes |
| source system | 128 UTF-8 bytes |
| source reference | 1,024 UTF-8 bytes |
| context attributes | 32 |
| attribute key | 128 UTF-8 bytes |
| attribute value | 1,024 UTF-8 bytes |
| artifact name | 255 UTF-8 bytes |
| policy, revision, and analyzer identifiers | 128 UTF-8 bytes |
| default artifact bytes | 67,108,864 |

NUL and other disallowed control characters are rejected in identity-bearing metadata.

## Format classification order

1. PE magic
2. ELF magic
3. Mach-O and universal-binary magic
4. ZIP signatures
5. PDF signature
6. OLE compound-document signature
7. shebang or approved script extension with valid text
8. bounded UTF-8 text
9. unknown

Classification is evidence, not a safety declaration.

## Error policy

- Invalid request: return a contract error; no analysis occurs.
- Invalid artifact: return an ingestion error; no analyzer receives bytes.
- Invalid or duplicate analyzer identifiers: reject engine construction.
- Analyzer failure: emit `tool_failure`, retain prior evidence, return `inconclusive`.
- Disallowed analyzer evidence category: replace it with `tool_failure`, return `inconclusive`.
- Dynamic request without dynamic adapter: emit foundation evidence, return `inconclusive`.
- Serialization failure: return the serialization error; do not mutate the bundle.

## Future ports

Dynamic execution, object storage, evidence signing, queueing, and Wardnet integration must be added behind explicit traits. They must not broaden the authority of this runtime to consumer verdicts.
