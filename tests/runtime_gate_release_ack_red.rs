#[cfg(target_os = "linux")]
mod linux {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };

    const READY_MARKER: &str = "QSR_GATE_READY\n";
    const RELEASED_MARKER: &str = "QSR_GATE_RELEASED\n";
    const RELEASE_TOKEN: &str = "release-token-for-runtime-gate-ack-red";

    #[test]
    fn runtime_gate_acknowledges_release_before_exec() {
        let mut child = Command::new(env!("CARGO_BIN_EXE_qsr_runtime_gate"))
            .args([RELEASE_TOKEN, "/bin/true"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("runtime gate should spawn");

        let stdout = child
            .stdout
            .take()
            .expect("runtime gate stdout should exist");
        let mut stdout = BufReader::new(stdout);
        let mut line = String::new();
        stdout
            .read_line(&mut line)
            .expect("runtime gate readiness marker should be readable");
        assert_eq!(line, READY_MARKER);

        let mut stdin = child.stdin.take().expect("runtime gate stdin should exist");
        stdin
            .write_all(format!("{RELEASE_TOKEN}\n").as_bytes())
            .expect("release token should be writable");
        stdin.flush().expect("release token should flush");
        drop(stdin);

        line.clear();
        stdout
            .read_line(&mut line)
            .expect("runtime gate release acknowledgement should be readable");
        assert_eq!(
            line, RELEASED_MARKER,
            "the trusted runtime gate must acknowledge the accepted one-time token before replacing itself with consumer code"
        );

        let status = child.wait().expect("runtime gate child should be reapable");
        assert!(
            status.success(),
            "released /bin/true should exit successfully"
        );
    }
}
