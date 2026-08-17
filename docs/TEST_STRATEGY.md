# Test Strategy

## Test-first contract

The first behavioral commit contains integration tests that fail because the production API does not exist. The implementation commit is accepted only after the same tests pass on the exact head.

## Test layers

### Contract tests

- enum wire codes;
- serde round trips;
- field limits and control characters;
- SHA-256 shape;
- evidence ordering;
- mandatory consumer verdict boundary;
- runtime boundary invariants.

### Ingestion tests

- exact byte preservation and known SHA-256;
- PE, ELF, Mach-O, ZIP, PDF, OLE, script, text, and unknown fixtures;
- empty input;
- artifact-size limit;
- name limit and control characters;
- invalid policy bounds.

Fixtures are synthetic headers and harmless text. Real malware is not committed.

### Runtime tests

- completed static profile;
- fail-closed Linux and Windows dynamic profiles;
- ordered custom analyzer findings;
- analyzer failure preservation;
- invalid request and artifact errors;
- invalid engine configuration;
- deterministic job and evidence IDs;
- pretty JSON without verdict fields.

### Property and fuzz expansion

Follow-on work adds arbitrary-byte ingestion properties, metadata fuzzing, parser-adapter fuzzing, archive-depth limits, decompression-ratio tests, and analyzer-output schema fuzzing.

## Coverage

CI targets 100% production lines, functions, and regions with `cargo-llvm-cov`. Branch coverage is also a release target; a pinned branch-coverage lane is added once exact toolchain support is proven. Generated macro internals and third-party code are not included in production-source coverage.

## Dynamic validation

A future dynamic release must test real benign and malicious samples in a private controlled corpus, including packed, delayed, anti-VM, credential-access, persistence, C2, ransomware-simulation, and clean enterprise software. Metrics include malicious recall, benign false-positive rate, abstention calibration, runtime escape count, and evidence completeness.
