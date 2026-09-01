//! Infrastructure adapters for sandbox execution.

mod application_service_backend;
mod bounded_command;
mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
