//! Caller-scoped application-service lifecycle coordination.

use std::{collections::BTreeMap, sync::Mutex};

use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    ApplicationServiceError, ApplicationServiceLease, ApplicationServiceRequest, CleanupReceipt,
};
use crate::IsolationPolicy;

const MAX_LEASE_OWNER_ID_BYTES: usize = 128;
const MAX_EXPIRED_CLEANUPS_PER_CALL: usize = 64;

/// Coordinator-owned caller/idempotency errors.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ApplicationServiceCoordinatorError {
    /// The authenticated caller identity could not be represented safely.
    #[error("invalid application-service lease owner identity")]
    InvalidLeaseOwnerId,
    /// The same caller/request identity was reused for different request content.
    #[error("application-service idempotency conflict")]
    IdempotencyConflict,
    /// An identical caller/request launch is already executing.
    #[error("application-service launch already in progress")]
    LaunchInProgress,
    /// Cleanup for the same active lease is already executing.
    #[error("application-service termination already in progress")]
    TerminationInProgress,
    /// The caller does not own an active lease matching the supplied receipt.
    #[error("application-service lease is unknown to this caller")]
    UnknownLease,
    /// The caller supplied a receipt that differs from the registered active lease.
    #[error("application-service lease receipt does not match active registry state")]
    LeaseMismatch,
    /// The process-local coordinator registry could not be accessed safely.
    #[error("application-service coordinator state is unavailable")]
    StateUnavailable,
    /// The application-service request or sandbox backend failed.
    #[error(transparent)]
    Backend(#[from] ApplicationServiceError),
}

/// Opaque authenticated-caller identity used to scope leases and idempotency.
///
/// The runtime does not authenticate this value. A transport adapter must
/// construct it only after it has verified the caller and mapped that caller to
/// a stable opaque identity. It is intentionally separate from the untrusted
/// [`ApplicationServiceRequest`] payload.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeaseOwnerId(String);

impl LeaseOwnerId {
    /// Validate and construct a bounded opaque lease-owner identity.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceCoordinatorError::InvalidLeaseOwnerId`] for
    /// empty, oversized, whitespace-bearing, non-ASCII, or unsupported identities.
    pub fn new(value: &str) -> Result<Self, ApplicationServiceCoordinatorError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_LEASE_OWNER_ID_BYTES
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'.' | b'_' | b':' | b'/' | b'@' | b'-')
            });
        if !valid {
            return Err(ApplicationServiceCoordinatorError::InvalidLeaseOwnerId);
        }
        Ok(Self(value.to_owned()))
    }

    /// Return the stable opaque owner identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Infrastructure port required by the application-service coordinator.
///
/// Implementations may use Podman, containerd, gVisor, or another reviewed
/// sandbox backend. Idempotency and caller ownership stay in the Supporting
/// `application_service` bounded context rather than in an infrastructure
/// adapter.
pub trait ApplicationServiceBackend: Send + Sync {
    /// Launch one validated isolated application service.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when the backend cannot establish
    /// the requested isolation and readiness contract.
    fn launch_at(
        &self,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceError>;

    /// Stop one active service and remove all runtime-owned isolation resources.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceError`] when cleanup cannot be proven.
    fn terminate_at(
        &self,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceError>;
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LeaseKey {
    lease_owner_id: LeaseOwnerId,
    request_id: String,
}

impl LeaseKey {
    fn new(lease_owner_id: &LeaseOwnerId, request_id: &str) -> Self {
        Self {
            lease_owner_id: lease_owner_id.clone(),
            request_id: request_id.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RegistryEntry {
    Launching {
        request_fingerprint: String,
    },
    Active {
        request_fingerprint: String,
        lease: ApplicationServiceLease,
    },
    Terminating {
        request_fingerprint: String,
        lease: ApplicationServiceLease,
    },
}

/// One bounded expired-lease cleanup outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpiredLeaseCleanupResult {
    lease_owner_id: LeaseOwnerId,
    request_id: String,
    lease: ApplicationServiceLease,
    result: Result<CleanupReceipt, ApplicationServiceError>,
}

impl ExpiredLeaseCleanupResult {
    /// Return the authenticated caller identity that owned the expired lease.
    #[must_use]
    pub const fn lease_owner_id(&self) -> &LeaseOwnerId {
        &self.lease_owner_id
    }

    /// Return the caller-scoped idempotency key of the expired lease.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the public lease receipt that was selected for cleanup.
    #[must_use]
    pub const fn lease(&self) -> &ApplicationServiceLease {
        &self.lease
    }

    /// Return the cleanup receipt or attributable backend failure.
    #[must_use]
    pub const fn result(&self) -> &Result<CleanupReceipt, ApplicationServiceError> {
        &self.result
    }
}

/// In-memory application-service coordinator enforcing caller ownership and idempotency.
///
/// This coordinator is process-local. It prevents duplicate launches and
/// cross-caller cleanup within one running runtime, but it is not durable crash
/// recovery. A later persistence slice must reconstruct and reconcile leases
/// after restart before claiming orphan recovery.
pub struct ApplicationServiceCoordinator<B> {
    backend: B,
    leases: Mutex<BTreeMap<LeaseKey, RegistryEntry>>,
}

impl<B> ApplicationServiceCoordinator<B>
where
    B: ApplicationServiceBackend,
{
    /// Construct a coordinator around one reviewed sandbox backend.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            leases: Mutex::new(BTreeMap::new()),
        }
    }

    /// Launch or replay one caller-scoped application-service request.
    ///
    /// An identical retry for an active lease returns that lease without
    /// invoking the backend again. Reusing the same caller/request identity for
    /// different immutable request or effective policy content fails closed.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceCoordinatorError`] for invalid requests,
    /// idempotency conflicts, concurrent duplicate launches, state failure, or
    /// backend isolation failures.
    pub fn launch_at(
        &self,
        lease_owner_id: &LeaseOwnerId,
        request: &ApplicationServiceRequest,
        policy: &IsolationPolicy,
        started_at_epoch_seconds: u64,
    ) -> Result<ApplicationServiceLease, ApplicationServiceCoordinatorError> {
        request.validate(policy)?;
        let key = LeaseKey::new(lease_owner_id, &request.request_id);
        let request_fingerprint = fingerprint_request_and_policy(request, policy);
        {
            let mut leases = self.lock_registry()?;
            match leases.get(&key) {
                Some(RegistryEntry::Launching {
                    request_fingerprint: existing,
                }) if existing == &request_fingerprint => {
                    return Err(ApplicationServiceCoordinatorError::LaunchInProgress);
                }
                Some(RegistryEntry::Active {
                    request_fingerprint: existing,
                    lease,
                }) if existing == &request_fingerprint => return Ok(lease.clone()),
                Some(RegistryEntry::Terminating {
                    request_fingerprint: existing,
                    ..
                }) if existing == &request_fingerprint => {
                    return Err(ApplicationServiceCoordinatorError::TerminationInProgress);
                }
                Some(_) => return Err(ApplicationServiceCoordinatorError::IdempotencyConflict),
                None => {
                    leases.insert(
                        key.clone(),
                        RegistryEntry::Launching {
                            request_fingerprint: request_fingerprint.clone(),
                        },
                    );
                }
            }
        }

        let lease = match self
            .backend
            .launch_at(request, policy, started_at_epoch_seconds)
        {
            Ok(lease) => lease,
            Err(error) => {
                self.lock_registry()?.remove(&key);
                return Err(error.into());
            }
        };

        self.lock_registry()?.insert(
            key,
            RegistryEntry::Active {
                request_fingerprint,
                lease: lease.clone(),
            },
        );
        Ok(lease)
    }

    /// Terminate one lease only when the caller owns the active registry entry.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceCoordinatorError::UnknownLease`] for a wrong
    /// owner or unknown lease before any backend cleanup operation is attempted.
    pub fn terminate_at(
        &self,
        lease_owner_id: &LeaseOwnerId,
        lease: &ApplicationServiceLease,
        terminated_at_epoch_seconds: u64,
    ) -> Result<CleanupReceipt, ApplicationServiceCoordinatorError> {
        let key = LeaseKey::new(lease_owner_id, lease.request_id());
        let (request_fingerprint, registered_lease) = {
            let mut leases = self.lock_registry()?;
            match leases.get(&key) {
                Some(RegistryEntry::Active {
                    request_fingerprint,
                    lease: registered_lease,
                }) => {
                    if registered_lease != lease {
                        return Err(ApplicationServiceCoordinatorError::LeaseMismatch);
                    }
                    let request_fingerprint = request_fingerprint.clone();
                    let registered_lease = registered_lease.clone();
                    leases.insert(
                        key.clone(),
                        RegistryEntry::Terminating {
                            request_fingerprint: request_fingerprint.clone(),
                            lease: registered_lease.clone(),
                        },
                    );
                    (request_fingerprint, registered_lease)
                }
                Some(RegistryEntry::Launching { .. }) => {
                    return Err(ApplicationServiceCoordinatorError::LaunchInProgress);
                }
                Some(RegistryEntry::Terminating { .. }) => {
                    return Err(ApplicationServiceCoordinatorError::TerminationInProgress);
                }
                None => return Err(ApplicationServiceCoordinatorError::UnknownLease),
            }
        };

        let result = self
            .backend
            .terminate_at(&registered_lease, terminated_at_epoch_seconds);
        self.finish_termination(&key, request_fingerprint, registered_lease, &result)?;
        result.map_err(Into::into)
    }

    /// Clean up at most 64 active leases whose expiry is not later than `now`.
    ///
    /// Failed cleanup remains registered as active so a later operator or
    /// cleanup pass can retry it. This process-local function does not claim
    /// crash/restart orphan recovery.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationServiceCoordinatorError::StateUnavailable`] if the
    /// in-memory registry cannot be accessed safely.
    pub fn cleanup_expired_at(
        &self,
        now_epoch_seconds: u64,
    ) -> Result<Vec<ExpiredLeaseCleanupResult>, ApplicationServiceCoordinatorError> {
        let candidates = {
            let mut leases = self.lock_registry()?;
            let candidates: Vec<(LeaseKey, String, ApplicationServiceLease)> = leases
                .iter()
                .filter_map(|(key, entry)| match entry {
                    RegistryEntry::Active {
                        request_fingerprint,
                        lease,
                    } if lease.expires_at_epoch_seconds() <= now_epoch_seconds => Some((
                        key.clone(),
                        request_fingerprint.clone(),
                        lease.clone(),
                    )),
                    _ => None,
                })
                .take(MAX_EXPIRED_CLEANUPS_PER_CALL)
                .collect();
            for (key, request_fingerprint, lease) in &candidates {
                leases.insert(
                    key.clone(),
                    RegistryEntry::Terminating {
                        request_fingerprint: request_fingerprint.clone(),
                        lease: lease.clone(),
                    },
                );
            }
            candidates
        };

        let mut outcomes = Vec::with_capacity(candidates.len());
        for (key, request_fingerprint, lease) in candidates {
            let result = self.backend.terminate_at(&lease, now_epoch_seconds);
            self.finish_termination(&key, request_fingerprint, lease.clone(), &result)?;
            outcomes.push(ExpiredLeaseCleanupResult {
                lease_owner_id: key.lease_owner_id,
                request_id: key.request_id,
                lease,
                result,
            });
        }
        Ok(outcomes)
    }

    fn lock_registry(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, BTreeMap<LeaseKey, RegistryEntry>>,
        ApplicationServiceCoordinatorError,
    > {
        self.leases
            .lock()
            .map_err(|_| ApplicationServiceCoordinatorError::StateUnavailable)
    }

    fn finish_termination(
        &self,
        key: &LeaseKey,
        request_fingerprint: String,
        lease: ApplicationServiceLease,
        result: &Result<CleanupReceipt, ApplicationServiceError>,
    ) -> Result<(), ApplicationServiceCoordinatorError> {
        let mut leases = self.lock_registry()?;
        if result.is_ok() {
            leases.remove(key);
        } else {
            leases.insert(
                key.clone(),
                RegistryEntry::Active {
                    request_fingerprint,
                    lease,
                },
            );
        }
        Ok(())
    }
}

fn fingerprint_request_and_policy(
    request: &ApplicationServiceRequest,
    policy: &IsolationPolicy,
) -> String {
    let mut hasher = Sha256::new();
    for component in [
        request.schema_version.as_str(),
        request.request_id.as_str(),
        request.image_reference.as_str(),
        request.protocol.as_str(),
        policy.policy_id.as_str(),
    ] {
        hasher.update(component.as_bytes());
        hasher.update([0]);
    }
    hasher.update(request.container_port.to_be_bytes());
    for argument in &request.command {
        hasher.update(argument.as_bytes());
        hasher.update([0]);
    }
    for value in [
        request.resources.memory_bytes,
        u64::from(request.resources.cpu_millicores),
        u64::from(request.resources.maximum_processes),
        u64::from(request.resources.lease_seconds),
        request.resources.tmpfs_bytes,
        policy.maximum_memory_bytes,
        u64::from(policy.maximum_cpu_millicores),
        u64::from(policy.maximum_processes),
        u64::from(policy.maximum_lease_seconds),
        policy.maximum_tmpfs_bytes,
        policy.readiness_timeout_millis,
        policy.readiness_poll_interval_millis,
        u64::from(policy.shutdown_grace_seconds),
        u64::from(policy.run_as_user_id),
        u64::from(policy.run_as_group_id),
    ] {
        hasher.update(value.to_be_bytes());
    }
    format!("{:x}", hasher.finalize())
}
