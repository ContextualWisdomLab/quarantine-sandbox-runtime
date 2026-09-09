//! Infrastructure adapters for sandbox execution.

mod application_service_backend;
mod bounded_command;
#[cfg(all(test, unix))]
mod bounded_command_concrete_tests;
mod podman {
    include!("podman.rs");
    #[cfg(unix)]
    include!("podman_runtime_gate_binding.rs");
}
#[cfg(unix)]
mod runtime_gate_artifact;

#[cfg(unix)]
pub use podman::RuntimeGatePodmanAdapter;
pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
#[cfg(unix)]
pub use runtime_gate_artifact::{RuntimeGateArtifact, RuntimeGateArtifactError};
