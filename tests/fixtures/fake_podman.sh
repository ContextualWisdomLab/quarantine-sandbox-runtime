#!/bin/sh
set -eu

CONFIG="${0}.config"
# The config is data-only and may be rewritten between test phases; the executable
# itself is immutable during the test run so parallel process creation cannot race
# with a newly-written executable inode.
. "$CONFIG"
printf '%s\n' "$*" >> "$LOG"

APP_INFO='{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"5.6.2"}}'
APP_CONTAINER='[{"Id":"fake-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532"},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":32}}]'
APP_NETWORK='[{"internal":true,"dns_enabled":false}]'
COMMAND_INFO='{"host":{"security":{"rootless":true,"seccompEnabled":true,"seccompProfilePath":"/usr/share/containers/seccomp.json","apparmorEnabled":true,"selinuxEnabled":false}},"version":{"Version":"6.1.0"}}'
COMMAND_CONTAINER='[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]'
COMMAND_CONTAINER_INVALID='[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":null,"BoundingCaps":null,"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":false,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"","Annotations":{"io.podman.annotations.userns":"auto"},"PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}}}]'
COMMAND_CONTAINER_HOST_UTS='[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"host","CgroupMode":"private","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]'
COMMAND_CONTAINER_HOST_CGROUP='[{"Id":"fake-command-container-id","AppArmorProfile":"containers-default","ProcessLabel":"","EffectiveCaps":[],"BoundingCaps":[],"Config":{"User":"65532:65532","Timeout":20},"HostConfig":{"ReadonlyRootfs":true,"Privileged":false,"SecurityOpt":["no-new-privileges"],"UsernsMode":"auto","PidMode":"private","IpcMode":"none","NetworkMode":"none","UTSMode":"private","CgroupMode":"host","Memory":268435456,"NanoCpus":1000000000,"PidsLimit":16,"Tmpfs":{"/tmp":"rw,noexec,nosuid,nodev,size=16777216"}},"Mounts":[]}]'
COMMAND_TOP='PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL
1 filter - - - - - containers-default (enforce)'

case "$MODE" in
  source_script)
    # Dynamic scenario bodies are sourced as data by this checked-in immutable
    # executable. No test spawns a file that it has just written or rewritten.
    . "$SCRIPT"
    ;;
  app_success|app_slow_rootless|app_fail_rootless)
    if [ "$MODE" = app_slow_rootless ] && [ "${1:-}" = info ]; then sleep 1; fi
    if [ "$MODE" = app_fail_rootless ] && [ "${1:-}" = info ]; then exit 20; fi
    case "${1:-}:${2:-}" in
      info:--format) printf '%s\n' "$APP_INFO" ;;
      network:create) : ;;
      network:inspect) printf '%s\n' "$APP_NETWORK" ;;
      network:rm) : ;;
      container:inspect) printf '%s\n' "$APP_CONTAINER" ;;
      top:*) printf 'PID SECCOMP CAPEFF CAPBND CAPINH CAPPRM CAPAMB LABEL\n1 filter 0000000000000000 0000000000000000 0000000000000000 0000000000000000 0000000000000000 containers-default (enforce)\n' ;;
      create:--name) printf 'fake-container-id\n' ;;
      start:*) : ;;
      port:*) printf '127.0.0.1:%s\n' "$READY_PORT" ;;
      stop:*) : ;;
      rm:*) : ;;
      *) exit 91 ;;
    esac
    ;;
  command_nonzero_logs|command_start_cleanup_fail|command_logs_cleanup_fail|command_logs_timeout_cleanup_fail|command_isolation_cleanup_fail|command_host_uts|command_host_cgroup)
    case "${1:-}:${2:-}" in
      info:--format) printf '%s\n' "$COMMAND_INFO" ;;
      create:--name) printf 'fake-command-container-id\n' ;;
      init:*) : ;;
      start:*)
        if [ "$MODE" = command_start_cleanup_fail ]; then exit 17; fi
        ;;
      container:inspect)
        case "$MODE" in
          command_isolation_cleanup_fail) printf '%s\n' "$COMMAND_CONTAINER_INVALID" ;;
          command_host_uts) printf '%s\n' "$COMMAND_CONTAINER_HOST_UTS" ;;
          command_host_cgroup) printf '%s\n' "$COMMAND_CONTAINER_HOST_CGROUP" ;;
          *) printf '%s\n' "$COMMAND_CONTAINER" ;;
        esac
        ;;
      top:*) printf '%s\n' "$COMMAND_TOP" ;;
      wait:*) printf '0\n' ;;
      logs:*)
        if [ "$MODE" = command_logs_timeout_cleanup_fail ]; then sleep 3; exit 0; fi
        printf 'podman logs backend failure\n' >&2
        exit 42
        ;;
      rm:--force)
        case "$MODE" in
          command_start_cleanup_fail|command_logs_cleanup_fail|command_logs_timeout_cleanup_fail|command_isolation_cleanup_fail) exit 88 ;;
          *) : ;;
        esac
        ;;
      *) exit 91 ;;
    esac
    ;;
  command_owned_lifecycle)
    mark_foreign() { printf 'foreign-container-touched' > "$FOREIGN_MARKER"; }
    require_owned() {
      [ "${1:-}" = "$OWNED_CONTAINER_ID" ] || { mark_foreign; exit 97; }
    }
    case "${1:-}:${2:-}" in
      info:--format) printf '%s\n' "$COMMAND_INFO" ;;
      create:--name) : > "$OWNED_MARKER"; printf '%s\n' "$OWNED_CONTAINER_ID" ;;
      init:*) require_owned "${2:-}" ;;
      start:*) require_owned "${2:-}" ;;
      container:inspect) require_owned "${5:-}"; printf '%s\n' "$OWNED_CONTAINER_INSPECT" ;;
      top:*) require_owned "${2:-}"; printf '%s\n' "$COMMAND_TOP" ;;
      wait:*) require_owned "${2:-}"; printf '0\n' ;;
      logs:*) require_owned "${2:-}"; printf 'owned stdout\n' ;;
      kill:*) require_owned "${2:-}" ;;
      rm:--force) require_owned "${4:-}"; rm -f "$OWNED_MARKER" ;;
      *) exit 91 ;;
    esac
    ;;
  *) exit 92 ;;
esac
