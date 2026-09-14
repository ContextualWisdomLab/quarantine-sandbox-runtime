#!/usr/bin/env python3
"""One-shot adoption of canonical application-service runtime identity semantics.

The canonical application-service owner (#21) proved that consumer correlation fields and
wall-clock time cannot safely name destructive runtime resources. This temporary fixer adapts
that owner contract onto the current command-runtime architecture and is removed by the source-fix
workflow after exact RED/GREEN validation.
"""

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one literal match, found {count}")
    target.write_text(text.replace(old, new, 1))


replace_once(
    "src/sandbox_execution/mod.rs",
    '''    /// Adding lease duration to the start timestamp overflowed.\n    #[error("application service lease expiry overflow")]\n    LeaseExpiryOverflow,\n''',
    '''    /// Adding lease duration to the start timestamp overflowed.\n    #[error("application service lease expiry overflow")]\n    LeaseExpiryOverflow,\n    /// The runtime could not obtain operating-system entropy for an invocation identity.\n    #[error("application service runtime identity entropy is unavailable")]\n    RuntimeIdentityUnavailable,\n''',
)

replace_once(
    "src/infrastructure/podman.rs",
    '''const DEFAULT_COMMAND_LOG_STORAGE_LIMIT_BYTES: usize = 1024 * 1024;\n''',
    '''const DEFAULT_COMMAND_LOG_STORAGE_LIMIT_BYTES: usize = 1024 * 1024;\nconst RUNTIME_IDENTITY_ENTROPY_BYTES: usize = 16;\nconst LOWER_HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";\n''',
)

replace_once(
    "src/infrastructure/podman.rs",
    '''    /// Build a deterministic fail-closed Podman launch plan without executing it.\n''',
    '''    /// Build a fail-closed Podman launch plan with a fresh runtime-owned identity.\n''',
)

replace_once(
    "src/infrastructure/podman.rs",
    '''        let identity = sandbox_identity(\n            &request.request_id,\n            &request.image_reference,\n            &policy.policy_id,\n            started_at_epoch_seconds,\n        );\n''',
    '''        let identity = runtime_identity()?;\n''',
)

runtime_identity = '''/// Obtain a fresh unpredictable application-service identity from operating-system entropy.\nfn runtime_identity() -> Result<String, ApplicationServiceError> {\n    runtime_identity_with(|entropy| getrandom::fill(entropy))\n}\n\n/// Encode injected entropy as a fixed-width lowercase runtime identity or fail closed.\nfn runtime_identity_with<E>(\n    fill_entropy: impl FnOnce(&mut [u8; RUNTIME_IDENTITY_ENTROPY_BYTES]) -> Result<(), E>,\n) -> Result<String, ApplicationServiceError> {\n    let mut entropy = [0_u8; RUNTIME_IDENTITY_ENTROPY_BYTES];\n    fill_entropy(&mut entropy).map_err(|_| ApplicationServiceError::RuntimeIdentityUnavailable)?;\n\n    let mut identity = String::with_capacity(RUNTIME_IDENTITY_ENTROPY_BYTES * 2);\n    for byte in entropy {\n        identity.push(char::from(LOWER_HEX_DIGITS[usize::from(byte >> 4)]));\n        identity.push(char::from(LOWER_HEX_DIGITS[usize::from(byte & 0x0f)]));\n    }\n    Ok(identity)\n}\n\n#[cfg(test)]\nmod runtime_identity_tests {\n    use super::{RUNTIME_IDENTITY_ENTROPY_BYTES, runtime_identity_with};\n    use crate::ApplicationServiceError;\n\n    #[test]\n    fn injected_entropy_has_stable_lower_hex_encoding() {\n        let identity = runtime_identity_with(|entropy| {\n            *entropy = [0xab; RUNTIME_IDENTITY_ENTROPY_BYTES];\n            Ok::<(), ()>(())\n        });\n\n        assert_eq!(identity, Ok("ab".repeat(RUNTIME_IDENTITY_ENTROPY_BYTES)));\n    }\n\n    #[test]\n    fn entropy_failure_is_typed_and_fail_closed() {\n        let result = runtime_identity_with(|_| Err::<(), ()>(()));\n        assert_eq!(\n            result,\n            Err(ApplicationServiceError::RuntimeIdentityUnavailable)\n        );\n    }\n}\n\n'''
replace_once(
    "src/infrastructure/podman.rs",
    '''fn sandbox_identity(\n''',
    runtime_identity + '''fn sandbox_identity(\n''',
)
