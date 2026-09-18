//! Infrastructure adapters for sandbox execution.

mod bounded_command;
#[cfg(all(test, unix))]
mod bounded_command_concrete_tests;
#[cfg(all(test, unix))]
mod bounded_command_deadline_tests;
mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
