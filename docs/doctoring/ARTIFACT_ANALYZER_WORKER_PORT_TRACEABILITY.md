# Artifact Analyzer Worker Port Traceability

Issue #69 / Draft #70 defines the backend-neutral Analyzer Worker Execution port required by ADR-0009. This document records contract and evidence authority only; it does not claim that an isolated worker or concrete runtime adapter exists.

## Current authority

- Parent artifact-analysis authority: #18 `c0647152ec052d82969b2ae078891e25e6d4d69a`.
- Causal predecessor: issue #49 / descendant `70b49a5020037165fae9f68edcf4762a2c5b1055`, native CI `34018611889`, coverage job `101446928192`, where an external analyzer reached the controller-host loopback socket.
- First safety repair: current #18 rejects externally supplied in-process analyzers with `IsolatedAnalyzerWorkerRequired`; it is not worker capability.
- Worker-port RED test: `tests/artifact_analysis_worker_port_contract.rs`, initial test-bearing `7b01e5ee349a263eb03df082f3eead2f6b8a2872`.
- Evidence-identity hardening `0cd3db3142d32c1086f6cbb942d31394d5e9efc7` requires bounded worker/backend/version identifiers and a lower-case isolation-policy SHA-256.
- Applied-budget hardening `5609fa1d13a1a83bbae91e22e11d45fa1ac66188` requires runtime-owned applied-budget evidence and rejection of CPU/RAM/PID/wall-time/scratch/output mismatches.
- Parent formatter repair was adopted without force push by two-parent commit `f863a8975220f85c404ba8627406eb87055b4ec0`; fresh compare resolved merge base exactly to current #18 with `behind_by=0`.
- Security review then found that the isolation-policy SHA-256 was only syntax-checked in the receipt. The request carried no expected digest, so any well-formed lower-case digest could pass. RED hardening `4118ce0ac93766d3ba7263009faefc7352e493f7` makes the request pin the expected isolation-policy digest and requires `validate_against` to reject a different receipt digest.
- Review of exact `cc75b4187792ffc83276be4306d4a2d01972b1cf` found a second contract false-GREEN: issue #69 says public identifiers are bounded and SHA-256 identities are immutable, but the existing RED did not reject oversized/control-bearing analyzer, policy, backend, and version text or 63-character/non-hex digests. RED hardening `a0bd5d1ce729fd2d332c32ebdd05d9577a1d191a` adds `tests/artifact_analysis_worker_identifier_contract.rs`, using the repository's existing 128-byte/control-free engine-identifier boundary and exact 64-character lower-case hexadecimal digest semantics.
- Native CI `34058534560` belongs to predecessor `cc75b418...`; its queued state does not transfer after the identifier/digest RED moved the branch.

## Contract boundary

`artifact_analysis` owns the analyzer-worker invocation contract: immutable analyzer implementation identity, exact immutable artifact identity, policy identity, expected isolation-policy digest, normalized finding/failure semantics, bounded result admission, and anti-contradiction validation between requested and runtime-reported worker evidence. `sandbox_execution` owns reusable isolation, resource, lifecycle, termination, cleanup, and effective-runtime evidence semantics. `infrastructure` owns Podman/gVisor/containerd/VM translation and evidence collection.

The proposed port therefore carries a stable logical analyzer identifier, implementation version, immutable implementation SHA-256, exact artifact SHA-256, policy identity, expected immutable isolation-policy SHA-256, and explicit CPU/RAM/PID/wall-time/scratch/output budgets. Human-readable analyzer/policy/worker/backend/version identifiers are bounded to 128 bytes and reject control text, matching the existing artifact-analysis engine identifier boundary. SHA-256 identities are exactly 64 lower-case hexadecimal characters. A worker receipt must bind request inputs, report the applied budget, and expose runtime-owned evidence for denied network, ambient credentials, broad host filesystem, runtime sockets, uncontrolled subprocess authority, and cleanup completion. `validate_against` must reject both requested/applied budget mismatches and an isolation-policy digest different from the request.

A Rust trait, fake adapter, serialized receipt, requested budget, matching receipt field, or matching policy digest is not confinement evidence. Controller-side `validate_against` is an anti-contradiction boundary; it cannot establish that a backend actually enforced a control. Release authority requires a concrete adapter plus real negative/positive E2E on the same immutable integrated source identity.

## RED and minimum GREEN

The RED intentionally references worker-port types that do not exist yet. The intended first failure is a compile-contract failure showing that ADR-0009's port has not landed. Production must not move until the current exact RED executes for the intended cause.

After causal RED execution, the smallest GREEN is the backend-neutral port/value-object implementation and validation required by the tests, including exact request/receipt isolation-policy binding, requested/applied budget binding, bounded/control-free identifier validation, and exact lower-case SHA-256 validation. It must not wire arbitrary external analyzers back into the controller process and must not add Podman-specific types to `artifact_analysis`.

Port-level GREEN still does not complete #49 or ADR-0009. A later adapter must prove effective network/environment/filesystem/runtime-socket/process denial, actual resource ceilings, termination and cleanup. Those observations must come from the runtime/backend boundary and be bound to the exact worker, analyzer implementation, artifact and policy.

## Standards and research

NIST SP 800-190 remains the current NIST application-container security guide. It treats container isolation and runtime controls as security mechanisms rather than software-interface conventions. The Open Container Initiative released Runtime Specification v1.3.0 on 2025-11-04; OCI's current Runtime Specification publication is dated November 2025. These sources justify a backend-neutral execution contract plus concrete runtime enforcement, but neither source makes a particular adapter sufficient without observed evidence.

Peer-reviewed systems research supports the same evidence boundary. Van't Hof and Nieh (2022) show that conventional containers rely on the operating system to enforce isolation and introduce a smaller security monitor specifically to strengthen that trust boundary. The relevance here is not that BlackBox is the selected backend; it is that an API declaration or worker self-report cannot substitute for enforcement at the actual isolation boundary. Consequently, #69 keeps request/receipt consistency separate from #49/ADR-0009 effective-runtime proof.

### References

Open Container Initiative. (2025, November 4). *OCI runtime-spec v. 1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Souppaya, M., Morello, J., & Scarfone, K. (2017). *Application container security guide* (NIST Special Publication 800-190). National Institute of Standards and Technology. https://doi.org/10.6028/NIST.SP.800-190

Van't Hof, A., & Nieh, J. (2022). BlackBox: A container security monitor for protecting containers on untrusted operating systems. In *16th USENIX Symposium on Operating Systems Design and Implementation (OSDI 22)* (pp. 683–700). USENIX Association. https://www.usenix.org/conference/osdi22/presentation/vant-hof
