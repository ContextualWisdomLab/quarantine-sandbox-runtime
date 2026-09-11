from pathlib import Path

path = Path("src/main.rs")
text = path.read_text()
old = '''    gate_source=''
    for argument in "$@"; do
      case "$argument" in
        --volume=*:/qsr-runtime-gate:ro)
          gate_source="${argument#--volume=}"
          gate_source="${gate_source%:/qsr-runtime-gate:ro}"
          ;;
      esac
    done
'''
new = '''    gate_source=''
    expect_volume_value=0
    for argument in "$@"; do
      if [ "$expect_volume_value" -eq 1 ]; then
        case "$argument" in
          *:/qsr-runtime-gate:ro)
            gate_source="${argument%:/qsr-runtime-gate:ro}"
            ;;
        esac
        expect_volume_value=0
      elif [ "$argument" = "--volume" ]; then
        expect_volume_value=1
      fi
    done
'''
count = text.count(old)
if count != 1:
    raise SystemExit(f"src/main.rs: expected one generated gate-volume parser, found {count}")
path.write_text(text.replace(old, new, 1))
