//! Infrastructure adapters for sandbox execution.

mod application_service_backend;
mod bounded_command;
#[cfg(all(test, unix))]
mod bounded_command_concrete_tests;
mod podman;
#[cfg(unix)]
mod podman_runtime_gate_binding;
#[cfg(unix)]
mod runtime_gate_artifact;

pub use podman::{PodmanLaunchPlan, RootlessPodmanAdapter};
#[cfg(unix)]
pub use podman_runtime_gate_binding::{RuntimeGateCommandBindingPlan, RuntimeGatePodmanAdapter};
#[cfg(unix)]
pub use runtime_gate_artifact::{RuntimeGateArtifact, RuntimeGateArtifactError};
