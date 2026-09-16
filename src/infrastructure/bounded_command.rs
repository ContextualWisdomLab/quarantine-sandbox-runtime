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

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
use rustix::process::{Pid, Signal, kill_process_group};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Internal failure classes preserved by the Podman adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoundedCommandError {
    /// The executable could not be spawned; preserve the OS failure class.
    Spawn(io::ErrorKind),
    /// The child could not be observed or reaped reliably.
    Wait,
    /// The child exceeded its wall-clock budget and was killed and reaped.
    Timeout,
    /// Stdout or stderr exceeded the configured retained-output budget.
    OutputLimit,
    /// A required output pipe was missing or a pipe-draining worker failed.
    Capture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptureWaitOutcome {
    Finished,
    Deadline,
    OutputLimit,
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
        let (mut child, stdout, stderr) = spawn_piped_child(program, args)?;
        let overflow = Arc::new(AtomicBool::new(false));
        let stdout_handle = drain_stream(stdout, self.output_limit_bytes, Arc::clone(&overflow));
        let stderr_handle = drain_stream(stderr, self.output_limit_bytes, Arc::clone(&overflow));
        let deadline = Instant::now() + self.timeout;

        let status_result = supervise_child(&mut child, deadline, overflow.as_ref());
        let status_result = enforce_capture_deadline(
            &mut child,
            &stdout_handle,
            &stderr_handle,
            deadline,
            overflow.as_ref(),
            status_result,
        );
        let stdout_result = join_stream(stdout_handle);
        let stderr_result = join_stream(stderr_handle);
        finalize_output(
            status_result,
            stdout_result,
            stderr_result,
            overflow.load(Ordering::Acquire),
        )
    }
}

fn spawn_piped_child(
    program: &Path,
    args: &[String],
) -> Result<(Child, ChildStdout, ChildStderr), BoundedCommandError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| BoundedCommandError::Spawn(error.kind()))?;
    captured_pipes(child.stdout.take(), child.stderr.take())
        .map(|(stdout, stderr)| (child, stdout, stderr))
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut Command) {}

fn captured_pipes<T, U>(
    stdout: Option<T>,
    stderr: Option<U>,
) -> Result<(T, U), BoundedCommandError> {
    match (stdout, stderr) {
        (Some(stdout), Some(stderr)) => Ok((stdout, stderr)),
        _ => Err(BoundedCommandError::Capture),
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
        terminate_child_process_group(self)
    }

    fn reap(&mut self) -> io::Result<ExitStatus> {
        self.wait()
    }
}

#[cfg(unix)]
fn terminate_child_process_group(child: &mut Child) -> io::Result<()> {
    kill_process_group(Pid::from_child(child), Signal::KILL).map_err(io::Error::from)
}

#[cfg(not(unix))]
fn terminate_child_process_group(child: &mut Child) -> io::Result<()> {
    child.kill()
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

fn enforce_capture_deadline<P, T, U>(
    child: &mut P,
    stdout_handle: &JoinHandle<T>,
    stderr_handle: &JoinHandle<U>,
    deadline: Instant,
    overflow: &AtomicBool,
    status_result: Result<ExitStatus, BoundedCommandError>,
) -> Result<ExitStatus, BoundedCommandError>
where
    P: ChildProcess,
{
    if status_result.is_err() {
        return status_result;
    }
    match streams_finished_before_deadline(stdout_handle, stderr_handle, deadline, overflow) {
        CaptureWaitOutcome::Finished => status_result,
        CaptureWaitOutcome::Deadline => {
            child.terminate().map_err(|_| BoundedCommandError::Wait)?;
            Err(BoundedCommandError::Timeout)
        }
        CaptureWaitOutcome::OutputLimit => {
            child.terminate().map_err(|_| BoundedCommandError::Wait)?;
            Err(BoundedCommandError::OutputLimit)
        }
    }
}

fn streams_finished_before_deadline<T, U>(
    stdout_handle: &JoinHandle<T>,
    stderr_handle: &JoinHandle<U>,
    deadline: Instant,
    overflow: &AtomicBool,
) -> CaptureWaitOutcome {
    loop {
        if stdout_handle.is_finished() && stderr_handle.is_finished() {
            return CaptureWaitOutcome::Finished;
        }
        if overflow.load(Ordering::Acquire) {
            return CaptureWaitOutcome::OutputLimit;
        }
        let now = Instant::now();
        if now >= deadline {
            return CaptureWaitOutcome::Deadline;
        }
        thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
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
            mpsc,
        },
        thread,
        time::{Duration, Instant},
    };

    use super::{
        BoundedCommandError, CaptureWaitOutcome, ChildProcess, captured_pipes, drain_stream,
        enforce_capture_deadline, finalize_output, join_stream, kill_and_reap,
        streams_finished_before_deadline, supervise_child,
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
            Err(BoundedCommandError::Capture)
        );
        assert_eq!(
            captured_pipes(Some(7_u8), None::<u8>),
            Err(BoundedCommandError::Capture)
        );
        assert_eq!(
            captured_pipes(None::<u8>, None::<u8>),
            Err(BoundedCommandError::Capture)
        );
    }

    #[test]
    fn supervision_preserves_exit_timeout_overflow_and_poll_failures() {
        let overflow = AtomicBool::new(false);
        let mut exited = FakeChild::new([PollOutcome::Exited]);
        assert!(supervise_child(&mut exited, Instant::now(), &overflow).is_ok());

        let mut eventually_exited = FakeChild::new([PollOutcome::Running, PollOutcome::Exited]);
        assert!(
            supervise_child(
                &mut eventually_exited,
                Instant::now() + Duration::from_millis(100),
                &overflow,
            )
            .is_ok()
        );

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
    fn capture_workers_share_the_command_deadline() {
        let overflow = AtomicBool::new(false);
        let completed_stdout = thread::spawn(|| ());
        let completed_stderr = thread::spawn(|| ());
        assert_eq!(
            streams_finished_before_deadline(
                &completed_stdout,
                &completed_stderr,
                Instant::now() + Duration::from_millis(100),
                &overflow,
            ),
            CaptureWaitOutcome::Finished
        );
        completed_stdout.join().expect("stdout worker must finish");
        completed_stderr.join().expect("stderr worker must finish");

        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let blocked_stdout = thread::spawn(move || receiver.recv());
        let completed_stderr = thread::spawn(|| ());
        assert_eq!(
            streams_finished_before_deadline(
                &blocked_stdout,
                &completed_stderr,
                Instant::now(),
                &overflow,
            ),
            CaptureWaitOutcome::Deadline
        );
        sender
            .send(())
            .expect("blocked stdout worker must be released");
        blocked_stdout
            .join()
            .expect("stdout worker must finish")
            .expect("release must arrive");
        completed_stderr.join().expect("stderr worker must finish");

        let overflow = AtomicBool::new(true);
        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let blocked_stdout = thread::spawn(move || receiver.recv());
        let completed_stderr = thread::spawn(|| ());
        assert_eq!(
            streams_finished_before_deadline(
                &blocked_stdout,
                &completed_stderr,
                Instant::now() + Duration::from_millis(100),
                &overflow,
            ),
            CaptureWaitOutcome::OutputLimit
        );
        sender
            .send(())
            .expect("blocked stdout worker must be released");
        blocked_stdout
            .join()
            .expect("stdout worker must finish")
            .expect("release must arrive");
        completed_stderr.join().expect("stderr worker must finish");
    }

    #[test]
    fn capture_deadline_preserves_prior_errors_and_cleanup_outcomes() {
        let overflow = AtomicBool::new(false);
        let completed_stdout = thread::spawn(|| ());
        let completed_stderr = thread::spawn(|| ());
        let mut prior_error = FakeChild::new([]);
        assert_eq!(
            enforce_capture_deadline(
                &mut prior_error,
                &completed_stdout,
                &completed_stderr,
                Instant::now(),
                &overflow,
                Err(BoundedCommandError::Wait),
            ),
            Err(BoundedCommandError::Wait)
        );
        completed_stdout.join().expect("stdout worker must finish");
        completed_stderr.join().expect("stderr worker must finish");

        let completed_stdout = thread::spawn(|| ());
        let completed_stderr = thread::spawn(|| ());
        let mut complete = FakeChild::new([]);
        assert!(
            enforce_capture_deadline(
                &mut complete,
                &completed_stdout,
                &completed_stderr,
                Instant::now() + Duration::from_millis(100),
                &overflow,
                Ok(success_status()),
            )
            .is_ok()
        );
        completed_stdout.join().expect("stdout worker must finish");
        completed_stderr.join().expect("stderr worker must finish");

        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let blocked_stdout = thread::spawn(move || receiver.recv());
        let completed_stderr = thread::spawn(|| ());
        let mut timed_out = FakeChild::new([]);
        assert_eq!(
            enforce_capture_deadline(
                &mut timed_out,
                &blocked_stdout,
                &completed_stderr,
                Instant::now(),
                &overflow,
                Ok(success_status()),
            ),
            Err(BoundedCommandError::Timeout)
        );
        sender
            .send(())
            .expect("blocked stdout worker must be released");
        blocked_stdout
            .join()
            .expect("stdout worker must finish")
            .expect("release must arrive");
        completed_stderr.join().expect("stderr worker must finish");

        let overflowed = AtomicBool::new(true);
        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let blocked_stdout = thread::spawn(move || receiver.recv());
        let completed_stderr = thread::spawn(|| ());
        let mut output_limited = FakeChild::new([]);
        assert_eq!(
            enforce_capture_deadline(
                &mut output_limited,
                &blocked_stdout,
                &completed_stderr,
                Instant::now() + Duration::from_millis(100),
                &overflowed,
                Ok(success_status()),
            ),
            Err(BoundedCommandError::OutputLimit)
        );
        sender
            .send(())
            .expect("blocked stdout worker must be released");
        blocked_stdout
            .join()
            .expect("stdout worker must finish")
            .expect("release must arrive");
        completed_stderr.join().expect("stderr worker must finish");

        let (sender, receiver) = mpsc::sync_channel::<()>(0);
        let blocked_stdout = thread::spawn(move || receiver.recv());
        let completed_stderr = thread::spawn(|| ());
        let mut cleanup_failed = FakeChild::new([]);
        cleanup_failed.terminate_ok = false;
        assert_eq!(
            enforce_capture_deadline(
                &mut cleanup_failed,
                &blocked_stdout,
                &completed_stderr,
                Instant::now(),
                &overflow,
                Ok(success_status()),
            ),
            Err(BoundedCommandError::Wait)
        );
        sender
            .send(())
            .expect("blocked stdout worker must be released");
        blocked_stdout
            .join()
            .expect("stdout worker must finish")
            .expect("release must arrive");
        completed_stderr.join().expect("stderr worker must finish");
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
