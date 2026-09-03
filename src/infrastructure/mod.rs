//! Infrastructure adapters for sandbox execution.

mod application_service_backend;
mod bounded_command;
#[cfg(all(test, unix))]
mod bounded_command_concrete_tests;
mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
