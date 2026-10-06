//! Offline static artifact-analysis command-line transport (ADR-0010).

use std::{io, process::ExitCode};

fn main() -> ExitCode {
    let code = quarantine_sandbox_runtime::run_static_analysis_cli(
        std::env::args_os().skip(1),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(code.as_u8())
}
