//! Offline application-service launch-plan command-line transport (ADR-0011).

use std::{io, process::ExitCode};

fn main() -> ExitCode {
    let exit = quarantine_sandbox_runtime::run_application_service_plan_cli(
        std::env::args_os().skip(1),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(exit.as_u8())
}
