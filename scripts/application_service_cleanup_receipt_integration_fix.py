#!/usr/bin/env python3
"""One-shot repair binding cleanup evidence to the exact removed runtime resources.

The public lease keeps a correlation-oriented sandbox name for compatibility, while destructive
cleanup uses private exact runtime authority. The cleanup receipt must report the latter or its
"removed" evidence is false. This temporary fixer runs only after cleanup authority is installed.
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
    "src/application_service/mod.rs",
    '''    pub(crate) fn complete(\n        lease: &ApplicationServiceLease,\n        terminated_at_epoch_seconds: u64,\n    ) -> Self {\n        Self {\n            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),\n            sandbox_id: lease.sandbox_id.clone(),\n            network_id: lease.network_id.clone(),\n            container_removed: true,\n            network_removed: true,\n            terminated_at_epoch_seconds,\n        }\n    }\n''',
    '''    pub(crate) fn complete(\n        removed_sandbox_id: &str,\n        removed_network_id: &str,\n        terminated_at_epoch_seconds: u64,\n    ) -> Self {\n        Self {\n            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),\n            sandbox_id: removed_sandbox_id.to_owned(),\n            network_id: removed_network_id.to_owned(),\n            container_removed: true,\n            network_removed: true,\n            terminated_at_epoch_seconds,\n        }\n    }\n''',
)

replace_once(
    "src/infrastructure/podman.rs",
    '''        Ok(CleanupReceipt::complete(lease, terminated_at_epoch_seconds))\n''',
    '''        Ok(CleanupReceipt::complete(\n            authority.sandbox_id(),\n            authority.network_id(),\n            terminated_at_epoch_seconds,\n        ))\n''',
)
