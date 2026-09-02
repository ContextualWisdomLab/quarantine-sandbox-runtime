//! Application-service backend port implementations owned by infrastructure.

use crate::{
    ApplicationServiceBackend, ApplicationServiceError, ApplicationServiceLease,
    ApplicationServiceRequest, CleanupReceipt, CommandExecutionBackend, CommandExecutionError,
    CommandExecutionRequest, CommandExecutionResult, IsolationPolicy, RootlessPodmanAdapter,
};

impl ApplicationServiceBackend for RootlessPodmanAdapter {
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError> {
        RootlessPodmanAdapter::launch_at(self, request, policy, started_at_epoch_seconds)
    }

    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError> {
        RootlessPodmanAdapter::terminate_at(self, lease, terminated_at_epoch_seconds)
    }
}

impl CommandExecutionBackend for RootlessPodmanAdapter {
    fn run_to_completion_at(
        &self,
        request: &CommandExecutionRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<CommandExecutionResult, CommandExecutionError> {
        RootlessPodmanAdapter::run_command_at(self, request, policy, started_at_epoch_seconds)
    }
}
