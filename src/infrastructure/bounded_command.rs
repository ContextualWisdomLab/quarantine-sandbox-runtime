//! Bounded subprocess execution for container-runtime command adapters.

use std::{
    io::{self, Read},
    path::Path,
    process::{Command, ExitStatus, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Internal failure classes preserved by the Podman adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundedCommandError {
    /// The executable could not be spawned or its pipes could not be captured.
    Spawn,
    /// The child could not be observed or reaped reliably.
    Wait,
    /// The child exceeded its wall-clock budget and was killed and reaped.
    Timeout,
    /// Stdout or stderr exceeded the configured retained-output budget.
    OutputLimit,
    /// A pipe-draining worker failed or panicked.
    Capture,
}

/// Execute direct argv with bounded wall-clock and retained stdout/stderr memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BoundedCommandRunner {
    timeout: Duration,
    output_limit_bytes: usize,
}

impl BoundedCommandRunner {
    /// Construct a runner with explicit command and per-stream output budgets.
    pub(crate) const fn new(timeout: Duration, output_limit_bytes: usize) -> Self {
        Self {
            timeout,
            output_limit_bytes,
        }
    }

    /// Execute one direct command, continuously draining both output pipes.
    ///
    /// Output beyond the configured per-stream limit is discarded while the
    /// child is terminated, preventing a hostile or defective CLI from filling
    /// OS pipes or growing retained diagnostics without bound.
    pub(crate) fn run(
        self,
        program: &Path,
        args: &[String],
    ) -> Result<Output, BoundedCommandError> {
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| BoundedCommandError::Spawn)?;
        let stdout = child.stdout.take().ok_or(BoundedCommandError::Spawn)?;
        let stderr = child.stderr.take().ok_or(BoundedCommandError::Spawn)?;
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout_handle = drain_stream(stdout, self.output_limit_bytes, Arc::clone(&overflow));
        let stderr_handle = drain_stream(stderr, self.output_limit_bytes, Arc::clone(&overflow));
        let deadline = Instant::now() + self.timeout;

        loop {
            if overflow.load(Ordering::Acquire) {
                kill_and_reap(&mut child)?;
                join_stream(stdout_handle)?;
                join_stream(stderr_handle)?;
                return Err(BoundedCommandError::OutputLimit);
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stdout = join_stream(stdout_handle)?;
                    let stderr = join_stream(stderr_handle)?;
                    if overflow.load(Ordering::Acquire) {
                        return Err(BoundedCommandError::OutputLimit);
                    }
                    return Ok(Output {
                        status,
                        stdout,
                        stderr,
                    });
                }
                Ok(None) => {
                    let now = Instant::now();
                    if now >= deadline {
                        kill_and_reap(&mut child)?;
                        join_stream(stdout_handle)?;
                        join_stream(stderr_handle)?;
                        return Err(BoundedCommandError::Timeout);
                    }
                    thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
                }
                Err(_) => {
                    kill_and_reap(&mut child)?;
                    join_stream(stdout_handle)?;
                    join_stream(stderr_handle)?;
                    return Err(BoundedCommandError::Wait);
                }
            }
        }
    }
}

fn drain_stream<R>(
    mut reader: R,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> JoinHandle<io::Result<Vec<u8>>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut retained = Vec::with_capacity(limit.min(8 * 1024));
        let mut buffer = [0_u8; 8 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(retained);
            }
            let remaining = limit.saturating_sub(retained.len());
            let keep = remaining.min(read);
            retained.extend_from_slice(&buffer[..keep]);
            if keep < read {
                overflow.store(true, Ordering::Release);
            }
        }
    })
}

fn join_stream(handle: JoinHandle<io::Result<Vec<u8>>>) -> Result<Vec<u8>, BoundedCommandError> {
    handle
        .join()
        .map_err(|_| BoundedCommandError::Capture)?
        .map_err(|_| BoundedCommandError::Capture)
}

fn kill_and_reap(child: &mut std::process::Child) -> Result<ExitStatus, BoundedCommandError> {
    if child.kill().is_err() && child.try_wait().map_err(|_| BoundedCommandError::Wait)?.is_none() {
        return Err(BoundedCommandError::Wait);
    }
    child.wait().map_err(|_| BoundedCommandError::Wait)
}
