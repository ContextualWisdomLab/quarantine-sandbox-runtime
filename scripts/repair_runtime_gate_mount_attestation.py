from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one exact replacement, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/infrastructure/podman_runtime_gate_binding.rs",
    """        self._inner.run_runtime_gate_command_at(
            request,
            policy,
            started_at_epoch_seconds,
            &binding_args,
""",
    """        self._inner.run_runtime_gate_command_at(
            request,
            policy,
            started_at_epoch_seconds,
            self.runtime_gate_artifact.path(),
            &binding_args,
""",
)

podman = "src/infrastructure/podman.rs"
replace_once(
    podman,
    """        self.run_command_with_binding_at(
            request,
            policy,
            started_at_epoch_seconds,
            None,
            |_| Ok(()),
        )
""",
    """        self.run_command_with_binding_at(
            request,
            policy,
            started_at_epoch_seconds,
            None,
            None,
            |_| Ok(()),
        )
""",
)
replace_once(
    podman,
    """        started_at_epoch_seconds: u64,
        runtime_gate_binding_args: &[String],
        release_gate: F,
""",
    """        started_at_epoch_seconds: u64,
        runtime_gate_artifact_path: &Path,
        runtime_gate_binding_args: &[String],
        release_gate: F,
""",
)
replace_once(
    podman,
    """            started_at_epoch_seconds,
            Some(runtime_gate_binding_args),
            release_gate,
""",
    """            started_at_epoch_seconds,
            Some(runtime_gate_artifact_path),
            Some(runtime_gate_binding_args),
            release_gate,
""",
)
replace_once(
    podman,
    """        _started_at_epoch_seconds: u64,
        runtime_gate_binding_args: Option<&[String]>,
        release_gate: F,
""",
    """        _started_at_epoch_seconds: u64,
        runtime_gate_artifact_path: Option<&Path>,
        runtime_gate_binding_args: Option<&[String]>,
        release_gate: F,
""",
)
replace_once(
    podman,
    """        if let Err(error) = self.verify_command_prestart_configuration(
            request,
            policy,
            &container_id,
            staged_source.as_ref().map(|staged| staged.path()),
        ) {
""",
    """        if let Err(error) = self.verify_command_prestart_configuration(
            request,
            policy,
            &container_id,
            staged_source.as_ref().map(|staged| staged.path()),
            runtime_gate_artifact_path,
        ) {
""",
)
replace_once(
    podman,
    """            &container_id,
            staged_source.as_ref().map(|staged| staged.path()),
        ) {
            return Err(self.cleanup_owned_command_container_or_report(&container_id, error.into()));
        }

        if runtime_gate_binding_args.is_some() {
""",
    """            &container_id,
            staged_source.as_ref().map(|staged| staged.path()),
            runtime_gate_artifact_path,
        ) {
            return Err(self.cleanup_owned_command_container_or_report(&container_id, error.into()));
        }

        if runtime_gate_binding_args.is_some() {
""",
)
replace_once(
    podman,
    """        container_id: &str,
        staged_source_path: Option<&Path>,
    ) -> Result<(), ApplicationServiceError> {
        let container = self.inspect_command_container(container_id)?;
        verify_command_container_configuration(request, policy, &container, staged_source_path)
""",
    """        container_id: &str,
        staged_source_path: Option<&Path>,
        runtime_gate_artifact_path: Option<&Path>,
    ) -> Result<(), ApplicationServiceError> {
        let container = self.inspect_command_container(container_id)?;
        verify_command_container_configuration(
            request,
            policy,
            &container,
            staged_source_path,
            runtime_gate_artifact_path,
        )
""",
)
replace_once(
    podman,
    """        container_id: &str,
        staged_source_path: Option<&Path>,
    ) -> Result<(), ApplicationServiceError> {
        let container = self.inspect_command_container(container_id)?;
        verify_command_container_configuration(request, policy, &container, staged_source_path)?;
""",
    """        container_id: &str,
        staged_source_path: Option<&Path>,
        runtime_gate_artifact_path: Option<&Path>,
    ) -> Result<(), ApplicationServiceError> {
        let container = self.inspect_command_container(container_id)?;
        verify_command_container_configuration(
            request,
            policy,
            &container,
            staged_source_path,
            runtime_gate_artifact_path,
        )?;
""",
)
replace_once(
    podman,
    """    container: &ContainerInspection,
    staged_source_path: Option<&Path>,
) -> Result<(), ApplicationServiceError> {
""",
    """    container: &ContainerInspection,
    staged_source_path: Option<&Path>,
    runtime_gate_artifact_path: Option<&Path>,
) -> Result<(), ApplicationServiceError> {
""",
)
replace_once(
    podman,
    """        container.mounts.len() == usize::from(request.source_artifact.is_some()),
    )?;
""",
    """        container.mounts.len()
            == usize::from(request.source_artifact.is_some())
                + usize::from(runtime_gate_artifact_path.is_some()),
    )?;
""",
)
replace_once(
    podman,
    """    if request.source_artifact.is_some() {
        let source_mount = container
""",
    """    if let Some(expected_gate_path) = runtime_gate_artifact_path {
        let runtime_gate_mount = container
            .mounts
            .iter()
            .find(|mount| mount.destination == "/qsr-runtime-gate");
        require_control(
            "runtime_gate_read_only",
            runtime_gate_mount.is_some_and(|mount| !mount.read_write),
        )?;
        require_control(
            "runtime_gate_bind_source",
            runtime_gate_mount.is_some_and(|mount| {
                mount.mount_type == "bind" && mount.source == expected_gate_path
            }),
        )?;
    }
    if request.source_artifact.is_some() {
        let source_mount = container
""",
)

integration_path = Path("tests/podman_runtime_gate_command_integration_red.rs")
integration = integration_path.read_text()
gate_marker = """    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let inspect = format!(
"""
gate_replacement = """    let gate = RuntimeGateArtifact::stage(&gate_source, &gate_sha256, std::env::consts::ARCH)
        .expect("verified self-contained gate should stage");
    let gate_path = gate.path().display().to_string();
    let inspect = format!(
"""
if integration.count(gate_marker) != 1:
    raise SystemExit("integration fixture gate marker changed")
integration = integration.replace(gate_marker, gate_replacement, 1)
inspect_start = integration.index("    let inspect = format!(\n")
inspect_end = integration.index("    let script = format!(\n", inspect_start)
new_inspect = r'''    let inspect = format!(
        "[{{\"Id\":\"{OWNED_CONTAINER_ID}\",\"AppArmorProfile\":\"containers-default\",\"ProcessLabel\":\"\",\"EffectiveCaps\":[],\"BoundingCaps\":[],\"Config\":{{\"User\":\"65532:65532\",\"Timeout\":20}},\"HostConfig\":{{\"ReadonlyRootfs\":true,\"Privileged\":false,\"SecurityOpt\":[\"no-new-privileges\"],\"UsernsMode\":\"auto\",\"PidMode\":\"private\",\"IpcMode\":\"none\",\"NetworkMode\":\"none\",\"UTSMode\":\"private\",\"CgroupMode\":\"private\",\"Memory\":268435456,\"NanoCpus\":1000000000,\"PidsLimit\":16,\"Tmpfs\":{{\"/tmp\":\"rw,noexec,nosuid,nodev,size=16777216\"}}}},\"Mounts\":[{{\"Source\":\"{gate_path}\",\"Destination\":\"/qsr-runtime-gate\",\"Type\":\"bind\",\"Options\":[\"ro\"],\"RW\":false}}]}}]"
    );
'''
integration_path.write_text(integration[:inspect_start] + new_inspect + integration[inspect_end:])

main_path = Path("src/main.rs")
main_text = main_path.read_text()
start_marker = "        const SUCCESS_SCRIPT: &str = "
end_marker = "\n\n        fn write_self_contained_gate"
start = main_text.find(start_marker)
end = main_text.find(end_marker, start)
if start == -1 or end == -1 or main_text.find(start_marker, start + 1) != -1:
    raise SystemExit("src/main.rs: could not locate unique SUCCESS_SCRIPT fixture")
success_script = r'''        const SUCCESS_SCRIPT: &str = r#"#!/bin/sh
set -eu
state="${0}.runtime-gate-source"
case "${1:-}:${2:-}" in
  info:--format)
    printf '%s\n' '{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/x","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}'
    ;;
  create:--name)
    gate_source=''
    for argument in "$@"; do
      case "$argument" in
        --volume=*:/qsr-runtime-gate:ro)
          gate_source="${argument#--volume=}"
          gate_source="${gate_source%:/qsr-runtime-gate:ro}"
          ;;
      esac
    done
    [ -n "$gate_source" ] || exit 92
    printf '%s' "$gate_source" > "$state"
    printf '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\n'
    ;;
  init:*) : ;;
  start:*) : ;;
  container:inspect)
    gate_source="$(cat "$state")"
    printf '%s\n' '[{"Id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":900},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":1073741824,"NanoCpus":4000000000,"PidsLimit":256,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=268435456"}},"Mounts":[{"Source":"'"$gate_source"'","Destination":"/qsr-runtime-gate","Type":"bind","Options":["ro"],"RW":false}]}]'
    ;;
  top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter - - - - - containers-default (enforce)\n' ;;
  attach:--sig-proxy=false) IFS= read -r token; [ -n "$token" ]; printf 'QSR_GATE_RELEASED\n'; while :; do :; done ;;
  wait:*) printf '9\n' ;;
  logs:*) printf 'cli stdout\n' ;;
  rm:--force) rm -f "$state" ;;
  *) exit 91 ;;
esac
"#;'''
main_path.write_text(main_text[:start] + success_script + main_text[end:])
