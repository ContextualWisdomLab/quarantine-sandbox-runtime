# Operability

## Foundation operating model

The foundation is a Rust library. A trusted host service is responsible for authentication, authorization, tenancy, request transport, queueing, and durable storage. The library performs bounded synchronous static processing only.

## Configuration

- artifact byte limit;
- artifact name byte limit;
- runtime policy identifier;
- source revision;
- ordered analyzer set.

Configuration errors prevent engine construction.

## Observability contract

Future host services should record:

- request acceptance and rejection counts;
- artifact sizes by bounded bucket;
- analysis latency;
- disposition counts;
- analyzer failure counts by producer and code;
- evidence-record counts;
- queue age and depth;
- worker allocation and destruction outcomes;
- network-policy violations;
- attestation-signing failures.

Do not put artifact bytes, prompt/response text, secrets, or unrestricted filenames in metrics.

## Reliability

- Same request and bytes are deterministic and idempotency-friendly.
- Analyzer failure does not discard identity or prior findings.
- Unsupported profiles are explicit and fail closed.
- A host may retry transport failures using request ID and artifact SHA-256.
- Consumers must not cache `inconclusive` as benign.

## Capacity

The default artifact limit is 64 MiB. It is a safety default, not a throughput claim. Future service-level capacity tests must cover concurrent ingestion, queue backpressure, analyzer CPU/RAM, object-storage bandwidth, worker startup, and worst-case archive expansion.

## Incident operations

If the runtime or analyzer supply chain is suspected:

1. stop scheduling affected profiles;
2. preserve request IDs, hashes, image digests, analyzer versions, and signed logs;
3. mark affected evidence as superseded or untrusted without deleting history;
4. rotate control-plane credentials even though workers should have none;
5. rebuild from pinned sources;
6. replay a private regression corpus;
7. publish a corrected evidence lineage.
