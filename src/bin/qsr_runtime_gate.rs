//! Runtime-owned hold gate for pre-payload isolation attestation.
//!
//! The gate is intentionally small: it becomes the initial OCI process, announces that the
//! consumer payload is held, accepts one bounded release token on standard input, and only
//! then replaces itself with the exact consumer program and argument vector. The sandbox
//! controller owns token generation and the release decision; hostile image content does not.

use std::{
    env,
    io::{self, Read, Write},
    os::unix::{ffi::OsStrExt, process::CommandExt},
    process::{self, Command, Stdio},
};

const READY_MARKER: &str = "QSR_GATE_READY\n";
const RELEASED_MARKER: &str = "QSR_GATE_RELEASED\n";
const MAX_RELEASE_TOKEN_BYTES: usize = 128;
const EXIT_INVALID_INVOCATION: i32 = 64;
const EXIT_RELEASE_REJECTED: i32 = 77;
const EXIT_EXEC_FAILED: i32 = 78;
const EXIT_CONTROL_CHANNEL_FAILED: i32 = 79;

/// Publishes one controller-visible marker and forces the shared stdout buffer out as one
/// fail-closed control-channel operation.
fn publish_control_marker(writer: &mut impl Write, marker: &str) -> io::Result<()> {
    writer.write_all(marker.as_bytes())?;
    writer.flush()
}

fn main() {
    let mut arguments = env::args_os().skip(1);
    let Some(expected_release_token) = arguments.next() else {
        process::exit(EXIT_INVALID_INVOCATION);
    };
    let Some(consumer_program) = arguments.next() else {
        process::exit(EXIT_INVALID_INVOCATION);
    };
    let consumer_arguments = arguments.collect::<Vec<_>>();

    if expected_release_token.as_os_str().as_bytes().is_empty()
        || expected_release_token.as_os_str().as_bytes().len() > MAX_RELEASE_TOKEN_BYTES
    {
        process::exit(EXIT_INVALID_INVOCATION);
    }

    let mut stdout = io::stdout().lock();
    if publish_control_marker(&mut stdout, READY_MARKER).is_err() {
        process::exit(EXIT_CONTROL_CHANNEL_FAILED);
    }

    let Some(release_token) = read_bounded_release_token() else {
        process::exit(EXIT_CONTROL_CHANNEL_FAILED);
    };
    if release_token != expected_release_token.as_os_str().as_bytes() {
        process::exit(EXIT_RELEASE_REJECTED);
    }

    if publish_control_marker(&mut stdout, RELEASED_MARKER).is_err() {
        process::exit(EXIT_CONTROL_CHANNEL_FAILED);
    }

    let error = Command::new(consumer_program)
        .args(consumer_arguments)
        .stdin(Stdio::null())
        .exec();
    eprintln!("qsr runtime gate exec failed: {}", error.kind());
    process::exit(EXIT_EXEC_FAILED);
}

fn read_bounded_release_token() -> Option<Vec<u8>> {
    let mut stdin = io::stdin().lock();
    let mut token = Vec::with_capacity(32);
    let mut byte = [0_u8; 1];

    loop {
        match stdin.read(&mut byte) {
            Ok(0) => return Some(token),
            Ok(_) if byte[0] == b'\n' => return Some(token),
            Ok(_) if token.len() < MAX_RELEASE_TOKEN_BYTES => token.push(byte[0]),
            Ok(_) => return None,
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    struct WriteFailure;

    impl Write for WriteFailure {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FlushFailure {
        written: Vec<u8>,
    }

    impl Write for FlushFailure {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(ErrorKind::BrokenPipe))
        }
    }

    #[test]
    fn control_marker_write_failure_is_preserved() {
        let mut writer = WriteFailure;
        assert_eq!(
            publish_control_marker(&mut writer, READY_MARKER).map_err(|error| error.kind()),
            Err(ErrorKind::BrokenPipe)
        );
    }

    #[test]
    fn control_marker_flush_failure_is_preserved_after_complete_write() {
        let mut writer = FlushFailure::default();
        assert_eq!(
            publish_control_marker(&mut writer, RELEASED_MARKER).map_err(|error| error.kind()),
            Err(ErrorKind::BrokenPipe)
        );
        assert_eq!(writer.written, RELEASED_MARKER.as_bytes());
    }
}
