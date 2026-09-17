# Command namespace fixture traceability

Status: candidate repair on PR #108, above canonical command-runtime PR #14. This is not protected integration, a release, or a real containment attestation. Existing ADR-0008 continues to own the command-runtime architecture; no new ADR number or sandbox implementation is introduced.

## Executed RED and root cause

Production `902377456036f19277ffe42fb24ce564d7dacaaf` requires the applied command namespace fields to be exactly `Some("private")`. Its predecessor `c05c8de29be1c22820848c5e9fab9b0218a54820` executed both missing-field regressions RED in run `34428925758`, branch-coverage job `102720035515`: absent evidence was previously admitted as successful consumer execution. That production repair is retained.

Decoded verify log for [run 34430833274 / job 102725806620](https://github.com/ContextualWisdomLab/quarantine-sandbox-runtime/actions/runs/34430833274/job/102725806620) exposes a distinct fixture failure on `902377456036f19277ffe42fb24ce564d7dacaaf`. The 36 library tests passed. The CLI target then reported 17 passed and one failed. `run_mirrors_the_sandboxed_exit_code_and_prints_json_on_a_successful_call` expected exit 9 but received 2; stderr was `effective isolation verification failed for isolated_uts_namespace`. The positive CLI fixture omitted both fields, so that job never reached the later namespace integration target. Lack of later execution is not a passing or failing result for those later tests.

The same omission exists in both positive inspection builders used by the primary command tests and in the shared `COMMAND_CONTAINER` used by cleanup scenarios. Repairing only the CLI would merely expose the next invalid positive fixture. The shared helper is repaired only at its explicitly named positive constant, never by normalizing arbitrary scenario output.

## Minimal delta and source identity

Commit `17860f342be63c97760ae9ea18ce340973c0d403` ordinary-fast-forwards `902377456036f19277ffe42fb24ce564d7dacaaf` and replaces three fixture lines in two files. Its successor in this lane adds the same two fields to the shared positive constant and records this operational evidence.

| File | Original Git blob | Candidate fixture Git blob |
|---|---|---|
| `src/main.rs` | `2fac7063c4698e875440d4331f8525bd028f511f` | `5f25f40d2a142d0c82becb69acdb38f5c75b7206` |
| `tests/podman_command_execution.rs` | `462cdca9402fd4e01cb7f434a51d0235a39f1196` | `74d8be12e76feb2bf883159399fbd5e0840e7fd2` |
| `tests/fixtures/fake_podman.sh` | `33fcddc4161324c6fea17a057f569b2b8478e458` | `d55aefd1bc48b694c97f49e299165a592d5bb346` |

Original source bytes were reconstructed from exact-revision GitHub reads and authenticated with Git blob framing. For each candidate, removing only the two inserted namespace fields reproduces the original bytes. The shared shell executable retains mode `100755`.

Unchanged: production namespace predicates; other namespace/resource/capability/mount checks; exact-ID cleanup and error precedence; CLI exit expectations; test assertions; intentionally absent/host namespace inputs; readonly-root failure data; workflow, dependencies, privileges and credentials. No retry, sleep, extra serialization, ignored test, test exclusion or host execution fallback is added.

## Measurements and acceptance

The local environment did not contain Cargo, rustc or Podman. Therefore no local Rust or real-container result is claimed. Local checks instead executed the CLI inspection shell and parsed both Rust-format inspection strings; the shared shell was syntax-checked and its seven command inspection modes were executed using private temporary config/log files. Four positive inspection builders now explicitly carry both required fields, versus zero of those four before repair. This is a fixture-completeness measure, not an attack-prevention rate. Host UTS, host cgroup and readonly-root counterexamples retain their original values.

Runbook and exact commands are maintained once in [OPERABILITY](../OPERABILITY.md#command-namespace-fixture-failures). Final acceptance requires the repaired CLI and primary/cleanup targets plus both missing-field regressions to execute successfully on one unchanged head. The missing-field cases must retain their exact error-control names, prove no `start` call, and prove cleanup targets only the acquired ID. Full repository/coverage/review/security and real positive-LSM gates remain independent.

The first fixture commit created native CI `34435812740`; its results are not inferred here. Use the live PR head and its own runs, not predecessor status, for any promotion. Root issue #25 still requires the complete runtime-owned hold → effective attestation → bounded one-time release → trusted ACK → detached consumer stdin → exact consumer exec lifecycle in the canonical production function. A fixture repair cannot satisfy that boundary.

## Primary references

ContextualWisdomLab. (2026, September 10). *Fail closed on missing command namespace evidence* [Pull request #108]. GitHub. https://github.com/ContextualWisdomLab/quarantine-sandbox-runtime/pull/108

ContextualWisdomLab. (2026, September 10). *CI verify log at 902377456036f19277ffe42fb24ce564d7dacaaf* [Workflow job 102725806620]. GitHub. https://github.com/ContextualWisdomLab/quarantine-sandbox-runtime/actions/runs/34430833274/job/102725806620

Podman Authors. (n.d.). *podman-create*. Podman documentation. Retrieved September 10, 2026, from https://docs.podman.io/en/latest/markdown/podman-create.1.html
