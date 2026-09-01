//! Infrastructure adapters for sandbox execution.

mod bounded_command;
mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
