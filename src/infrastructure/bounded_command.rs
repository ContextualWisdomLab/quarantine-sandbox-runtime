//! Bounded subprocess execution for container-runtime command adapters.

use std::{
    io::{self, Read},
    path::Path,
    process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Output, Stdio},
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

/// Terminal facts for a command run to completion under bounded wall-clock and
/// output budgets, where exceeding either budget is an expected, reportable
/// outcome rather than a hard error.
///
/// Contrast with [`BoundedCommandRunner::run`], whose administrative CLI calls
/// (inspect a JSON payload, create a resource) treat any overflow as an
/// anomaly: a well-behaved Podman CLI never legitimately produces more than a
/// few kilobytes of JSON, so overflow there indicates a malfunctioning or
/// hostile backend. A workload's own stdout/stderr has no such ceiling on
/// legitimate size, and a workload exceeding its wall-clock lease is routine,
/// so this variant reports both as facts on a successful outcome instead of
/// discarding the partial evidence collected before termination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundedRunOutcome {
    /// The process's own exit status, or `None` when it was killed before
    /// reporting one (wall-clock timeout or output-budget enforcement).
    pub(crate) status: Option<ExitStatus>,
    /// Whether the process was killed for exceeding its wall-clock budget.
    pub(crate) timed_out: bool,
    /// Standard output retained up to the configured per-stream budget.
    pub(crate) stdout: Vec<u8>,
    /// Whether standard output was truncated to the configured budget.
    pub(crate) stdout_truncated: bool,
    /// Standard error retained up to the configured per-stream budget.
    pub(crate) stderr: Vec<u8>,
    /// Whether standard error was truncated to the configured budget.
    pub(crate) stderr_truncated: bool,
}

type ExecuteOutcome = (
    Result<ExitStatus, BoundedCommandError>,
    Result<Vec<u8>, BoundedCommandError>,
    Result<Vec<u8>, BoundedCommandError>,
    bool,
);

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

    /// Spawn, supervise to a terminal state, and drain both pipes.
    ///
    /// Shared by [`Self::run`] and [`Self::run_to_completion`], which differ
    /// only in how they interpret a timeout or output-budget overflow.
    fn execute(
        self,
        program: &Path,
        args: &[String],
    ) -> Result<ExecuteOutcome, BoundedCommandError> {
        let (mut child, stdout, stderr) = spawn_piped_child(program, args)?;
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout_handle = drain_stream(stdout, self.output_limit_bytes, Arc::clone(&overflow));
        let stderr_handle = drain_stream(stderr, self.output_limit_bytes, Arc::clone(&overflow));
        let deadline = Instant::now() + self.timeout;

        let status_result = supervise_child(&mut child, deadline, overflow.as_ref());
        let stdout_result = join_stream(stdout_handle);
        let stderr_result = join_stream(stderr_handle);
        Ok((
            status_result,
            stdout_result,
            stderr_result,
            overflow.load(Ordering::Acquire),
        ))
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
        let (status_result, stdout_result, stderr_result, overflowed) =
            self.execute(program, args)?;
        finalize_output(status_result, stdout_result, stderr_result, overflowed)
    }

    /// Run one workload command to completion, reporting a wall-clock timeout
    /// or output-budget overflow as terminal facts instead of hard errors.
    ///
    /// A pipe-capture failure ([`BoundedCommandError::Capture`]) or an
    /// inability to observe/reap the child ([`BoundedCommandError::Wait`])
    /// remain hard errors: those indicate the supervising process itself
    /// malfunctioned, not a fact about the supervised workload.
    pub(crate) fn run_to_completion(
        self,
        program: &Path,
        args: &[String],
    ) -> Result<BoundedRunOutcome, BoundedCommandError> {
        let (status_result, stdout_result, stderr_result, overflowed) =
            self.execute(program, args)?;
        let stdout = stdout_result?;
        let stderr = stderr_result?;
        let (status, timed_out) = classify_completion_status(status_result)?;
        Ok(BoundedRunOutcome {
            status,
            timed_out,
            stdout,
            stdout_truncated: overflowed,
            stderr,
            stderr_truncated: overflowed,
        })
    }
}

fn spawn_piped_child(
    program: &Path,
    args: &[String],
) -> Result<(Child, ChildStdout, ChildStderr), BoundedCommandError> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| BoundedCommandError::Spawn)?;
    captured_pipes(child.stdout.take(), child.stderr.take())
        .map(|(stdout, stderr)| (child, stdout, stderr))
}

fn captured_pipes<T, U>(
    stdout: Option<T>,
    stderr: Option<U>,
) -> Result<(T, U), BoundedCommandError> {
    match (stdout, stderr) {
        (Some(stdout), Some(stderr)) => Ok((stdout, stderr)),
        _ => Err(BoundedCommandError::Spawn),
    }
}

fn finalize_output(
    status_result: Result<ExitStatus, BoundedCommandError>,
    stdout_result: Result<Vec<u8>, BoundedCommandError>,
    stderr_result: Result<Vec<u8>, BoundedCommandError>,
    overflowed: bool,
) -> Result<Output, BoundedCommandError> {
    let status = status_result?;
    let stdout = stdout_result?;
    let stderr = stderr_result?;
    if overflowed {
        return Err(BoundedCommandError::OutputLimit);
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Interpret the supervisor's terminal status for [`BoundedCommandRunner::run_to_completion`].
///
/// A wall-clock timeout or output-budget overflow become terminal facts
/// (`None` status, `timed_out` set only for the former); any other
/// supervision failure (an inability to observe or reap the child) remains a
/// hard error, since that indicates the supervisor itself malfunctioned, not
/// a fact about the supervised workload.
fn classify_completion_status(
    status_result: Result<ExitStatus, BoundedCommandError>,
) -> Result<(Option<ExitStatus>, bool), BoundedCommandError> {
    match status_result {
        Ok(status) => Ok((Some(status), false)),
        Err(BoundedCommandError::Timeout) => Ok((None, true)),
        Err(BoundedCommandError::OutputLimit) => Ok((None, false)),
        Err(other) => Err(other),
    }
}

trait ChildProcess {
    fn poll(&mut self) -> io::Result<Option<ExitStatus>>;
    fn terminate(&mut self) -> io::Result<()>;
    fn reap(&mut self) -> io::Result<ExitStatus>;
}

impl ChildProcess for Child {
    fn poll(&mut self) -> io::Result<Option<ExitStatus>> {
        self.try_wait()
    }

    fn terminate(&mut self) -> io::Result<()> {
        self.kill()
    }

    fn reap(&mut self) -> io::Result<ExitStatus> {
        self.wait()
    }
}

fn supervise_child<P: ChildProcess>(
    child: &mut P,
    deadline: Instant,
    overflow: &AtomicBool,
) -> Result<ExitStatus, BoundedCommandError> {
    loop {
        if overflow.load(Ordering::Acquire) {
            kill_and_reap(child)?;
            return Err(BoundedCommandError::OutputLimit);
        }
        match child.poll() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    kill_and_reap(child)?;
                    return Err(BoundedCommandError::Timeout);
                }
                thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
            }
            Err(_) => {
                kill_and_reap(child)?;
                return Err(BoundedCommandError::Wait);
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

fn kill_and_reap<P: ChildProcess>(child: &mut P) -> Result<ExitStatus, BoundedCommandError> {
    if child.terminate().is_ok() {
        return child.reap().map_err(|_| BoundedCommandError::Wait);
    }
    match child.poll().map_err(|_| BoundedCommandError::Wait)? {
        Some(status) => Ok(status),
        None => Err(BoundedCommandError::Wait),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        collections::VecDeque,
        io::{self, Cursor, Read},
        os::unix::process::ExitStatusExt,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Instant,
    };

    use super::{
        BoundedCommandError, ChildProcess, captured_pipes, classify_completion_status,
        drain_stream, finalize_output, join_stream, kill_and_reap, supervise_child,
    };

    #[derive(Clone, Copy)]
    enum PollOutcome {
        Running,
        Exited,
        Failed,
    }

    struct FakeChild {
        polls: VecDeque<PollOutcome>,
        terminate_ok: bool,
        reap_ok: bool,
    }

    impl FakeChild {
        fn new(polls: impl IntoIterator<Item = PollOutcome>) -> Self {
            Self {
                polls: polls.into_iter().collect(),
                terminate_ok: true,
                reap_ok: true,
            }
        }
    }

    impl ChildProcess for FakeChild {
        fn poll(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
            match self.polls.pop_front().unwrap_or(PollOutcome::Running) {
                PollOutcome::Running => Ok(None),
                PollOutcome::Exited => Ok(Some(success_status())),
                PollOutcome::Failed => Err(io::Error::other("poll failed")),
            }
        }

        fn terminate(&mut self) -> io::Result<()> {
            if self.terminate_ok {
                Ok(())
            } else {
                Err(io::Error::other("terminate failed"))
            }
        }

        fn reap(&mut self) -> io::Result<std::process::ExitStatus> {
            if self.reap_ok {
                Ok(success_status())
            } else {
                Err(io::Error::other("reap failed"))
            }
        }
    }

    struct ErrorReader;

    impl Read for ErrorReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read failed"))
        }
    }

    struct PanicReader;

    impl Read for PanicReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            std::panic::resume_unwind(Box::new("reader panic"))
        }
    }

    fn success_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }

    #[test]
    fn captured_pipes_require_both_configured_streams() {
        assert_eq!(captured_pipes(Some(7_u8), Some(8_u8)), Ok((7, 8)));
        assert_eq!(
            captured_pipes(None::<u8>, Some(8_u8)),
            Err(BoundedCommandError::Spawn)
        );
        assert_eq!(
            captured_pipes(Some(7_u8), None::<u8>),
            Err(BoundedCommandError::Spawn)
        );
        assert_eq!(
            captured_pipes(None::<u8>, None::<u8>),
            Err(BoundedCommandError::Spawn)
        );
    }

    #[test]
    fn supervision_preserves_exit_timeout_overflow_and_poll_failures() {
        let overflow = AtomicBool::new(false);
        let mut exited = FakeChild::new([PollOutcome::Exited]);
        assert!(supervise_child(&mut exited, Instant::now(), &overflow).is_ok());

        let mut timed_out = FakeChild::new([PollOutcome::Running]);
        assert_eq!(
            supervise_child(&mut timed_out, Instant::now(), &overflow),
            Err(BoundedCommandError::Timeout)
        );

        let overflow = AtomicBool::new(true);
        let mut noisy = FakeChild::new([]);
        assert_eq!(
            supervise_child(&mut noisy, Instant::now(), &overflow),
            Err(BoundedCommandError::OutputLimit)
        );

        let overflow = AtomicBool::new(false);
        let mut failed = FakeChild::new([PollOutcome::Failed]);
        assert_eq!(
            supervise_child(&mut failed, Instant::now(), &overflow),
            Err(BoundedCommandError::Wait)
        );
    }

    #[test]
    fn supervision_propagates_cleanup_failures_for_every_abort_path() {
        let overflow = AtomicBool::new(true);
        let mut overflow_cleanup_failed = FakeChild::new([]);
        overflow_cleanup_failed.reap_ok = false;
        assert_eq!(
            supervise_child(&mut overflow_cleanup_failed, Instant::now(), &overflow,),
            Err(BoundedCommandError::Wait)
        );

        let overflow = AtomicBool::new(false);
        let mut timeout_cleanup_failed = FakeChild::new([PollOutcome::Running]);
        timeout_cleanup_failed.reap_ok = false;
        assert_eq!(
            supervise_child(&mut timeout_cleanup_failed, Instant::now(), &overflow,),
            Err(BoundedCommandError::Wait)
        );

        let mut poll_cleanup_failed = FakeChild::new([PollOutcome::Failed]);
        poll_cleanup_failed.reap_ok = false;
        assert_eq!(
            supervise_child(&mut poll_cleanup_failed, Instant::now(), &overflow),
            Err(BoundedCommandError::Wait)
        );
    }

    #[test]
    fn finalized_output_preserves_late_overflow_and_capture_error_precedence() {
        assert_eq!(
            finalize_output(
                Ok(success_status()),
                Ok(b"stdout".to_vec()),
                Ok(b"stderr".to_vec()),
                false,
            )
            .map(|output| (output.stdout, output.stderr)),
            Ok((b"stdout".to_vec(), b"stderr".to_vec()))
        );

        assert_eq!(
            finalize_output(Ok(success_status()), Ok(Vec::new()), Ok(Vec::new()), true,),
            Err(BoundedCommandError::OutputLimit)
        );
        assert_eq!(
            finalize_output(
                Err(BoundedCommandError::Wait),
                Ok(Vec::new()),
                Ok(Vec::new()),
                false,
            ),
            Err(BoundedCommandError::Wait)
        );
        assert_eq!(
            finalize_output(
                Ok(success_status()),
                Err(BoundedCommandError::Capture),
                Ok(Vec::new()),
                false,
            ),
            Err(BoundedCommandError::Capture)
        );
        assert_eq!(
            finalize_output(
                Ok(success_status()),
                Ok(Vec::new()),
                Err(BoundedCommandError::Capture),
                false,
            ),
            Err(BoundedCommandError::Capture)
        );
    }

    #[test]
    fn classify_completion_status_reports_timeout_and_overflow_as_terminal_facts() {
        assert_eq!(
            classify_completion_status(Ok(success_status())),
            Ok((Some(success_status()), false))
        );
        assert_eq!(
            classify_completion_status(Err(BoundedCommandError::Timeout)),
            Ok((None, true))
        );
        assert_eq!(
            classify_completion_status(Err(BoundedCommandError::OutputLimit)),
            Ok((None, false))
        );
    }

    #[test]
    fn classify_completion_status_preserves_a_genuine_supervision_failure() {
        assert_eq!(
            classify_completion_status(Err(BoundedCommandError::Wait)),
            Err(BoundedCommandError::Wait)
        );
        assert_eq!(
            classify_completion_status(Err(BoundedCommandError::Spawn)),
            Err(BoundedCommandError::Spawn)
        );
    }

    #[test]
    fn kill_and_reap_distinguishes_reaped_running_and_unreapable_children() {
        let mut killed = FakeChild::new([]);
        assert!(kill_and_reap(&mut killed).is_ok());

        let mut reap_failed = FakeChild::new([]);
        reap_failed.reap_ok = false;
        assert_eq!(
            kill_and_reap(&mut reap_failed),
            Err(BoundedCommandError::Wait)
        );

        let mut already_exited = FakeChild::new([PollOutcome::Exited]);
        already_exited.terminate_ok = false;
        assert!(kill_and_reap(&mut already_exited).is_ok());

        let mut still_running = FakeChild::new([PollOutcome::Running]);
        still_running.terminate_ok = false;
        assert_eq!(
            kill_and_reap(&mut still_running),
            Err(BoundedCommandError::Wait)
        );

        let mut unobservable = FakeChild::new([PollOutcome::Failed]);
        unobservable.terminate_ok = false;
        assert_eq!(
            kill_and_reap(&mut unobservable),
            Err(BoundedCommandError::Wait)
        );
    }

    #[test]
    fn stream_workers_preserve_bounds_and_surface_reader_failures() {
        let overflow = Arc::new(AtomicBool::new(false));
        let handle = drain_stream(Cursor::new(b"safe".to_vec()), 4, Arc::clone(&overflow));
        assert_eq!(join_stream(handle), Ok(b"safe".to_vec()));
        assert!(!overflow.load(Ordering::Acquire));

        let overflow = Arc::new(AtomicBool::new(false));
        let handle = drain_stream(Cursor::new(b"overflow".to_vec()), 4, Arc::clone(&overflow));
        assert_eq!(join_stream(handle), Ok(b"over".to_vec()));
        assert!(overflow.load(Ordering::Acquire));

        let overflow = Arc::new(AtomicBool::new(false));
        assert_eq!(
            join_stream(drain_stream(ErrorReader, 4, overflow)),
            Err(BoundedCommandError::Capture)
        );

        let overflow = Arc::new(AtomicBool::new(false));
        assert_eq!(
            join_stream(drain_stream(PanicReader, 4, overflow)),
            Err(BoundedCommandError::Capture)
        );
    }
}
