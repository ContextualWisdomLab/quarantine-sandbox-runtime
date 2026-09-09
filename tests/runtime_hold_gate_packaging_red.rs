//! RED for immutable runtime-owned hold-gate packaging.
//!
//! A real hold/attest/release boundary cannot depend on compiling an ad-hoc CI fixture at
//! execution time. The package must expose a dedicated runtime-owned gate executable whose
//! release protocol fails closed and preserves the exact consumer argv after authorization.

#[cfg(unix)]
mod unix {
    use std::{
        fs,
        io::Write,
        process::{Command, Stdio},
    };

    #[test]
    fn package_exposes_fail_closed_runtime_gate_with_exact_consumer_argv() {
        let Some(gate_path) = option_env!("CARGO_BIN_EXE_qsr_runtime_gate") else {
            panic!("quarantine-sandbox-runtime must package the qsr_runtime_gate binary");
        };
        let workspace = tempfile::tempdir().expect("test workspace must be creatable");
        let sentinel_path = workspace.path().join("consumer-ran");
        let sentinel_text = "exact argv sentinel with spaces";
        let consumer_script = "printf '%s' \"$1\" > \"$2\"";
        let sentinel_path_text = sentinel_path
            .to_str()
            .expect("temporary test path must be Unicode");

        let rejected = Command::new(gate_path)
            .args([
                "expected-release-token",
                "/bin/sh",
                "-c",
                consumer_script,
                "qsr-consumer",
                sentinel_text,
                sentinel_path_text,
            ])
            .output()
            .expect("packaged gate must be spawnable");
        assert_eq!(
            rejected.status.code(),
            Some(77),
            "missing release authorization must fail closed"
        );
        assert!(
            !sentinel_path.exists(),
            "consumer payload must remain unexecuted without the exact release token"
        );
        assert!(
            String::from_utf8(rejected.stdout)
                .expect("gate readiness output must be UTF-8")
                .contains("QSR_GATE_READY"),
            "gate must make the held state observable before authorization"
        );

        let mut released = Command::new(gate_path)
            .args([
                "expected-release-token",
                "/bin/sh",
                "-c",
                consumer_script,
                "qsr-consumer",
                sentinel_text,
                sentinel_path_text,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("packaged gate must be spawnable");
        released
            .stdin
            .as_mut()
            .expect("release channel must be writable")
            .write_all(b"expected-release-token\n")
            .expect("release token must be writable");
        drop(released.stdin.take());
        let released = released
            .wait_with_output()
            .expect("released gate must be waitable");

        assert!(
            released.status.success(),
            "authorized gate must exec the consumer successfully: {}",
            String::from_utf8_lossy(&released.stderr)
        );
        assert!(
            String::from_utf8(released.stdout)
                .expect("gate readiness output must be UTF-8")
                .contains("QSR_GATE_READY"),
            "authorized execution must still expose the held-state readiness marker"
        );
        assert_eq!(
            fs::read_to_string(&sentinel_path).expect("consumer sentinel must be written"),
            sentinel_text,
            "release must preserve the exact consumer argument rather than shell-joining it"
        );
    }
}
