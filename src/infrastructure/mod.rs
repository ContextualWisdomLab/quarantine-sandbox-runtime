//! Infrastructure adapters for sandbox execution.

mod podman;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
