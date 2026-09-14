#!/usr/bin/env python3
"""One-shot repair binding cleanup evidence to the exact removed container.

The public lease keeps correlation-oriented sandbox and network identifiers for compatibility.
Destructive container cleanup uses private exact runtime authority, so the cleanup receipt must
report that acquired container ID instead of the mutable `qsr-app-*` name. Network lifecycle
identity remains the canonical #48 owner gap: the current adapter still addresses the generated
`qsr-net-*` correlation reference, so this fixer deliberately preserves the lease `network_id`
and does not misrepresent it as an acquired Podman network `.ID`.
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
    '''    pub(crate) fn complete(\n        removed_sandbox_id: &str,\n        lease: &ApplicationServiceLease,\n        terminated_at_epoch_seconds: u64,\n    ) -> Self {\n        Self {\n            schema_version: CONTRACT_SCHEMA_VERSION.to_owned(),\n            sandbox_id: removed_sandbox_id.to_owned(),\n            network_id: lease.network_id.clone(),\n            container_removed: true,\n            network_removed: true,\n            terminated_at_epoch_seconds,\n        }\n    }\n''',
)

replace_once(
    "src/infrastructure/podman.rs",
    '''        Ok(CleanupReceipt::complete(lease, terminated_at_epoch_seconds))\n''',
    '''        Ok(CleanupReceipt::complete(\n            authority.sandbox_id(),\n            lease,\n            terminated_at_epoch_seconds,\n        ))\n''',
)

replace_once(
    "src/application_service/coordinator.rs",
    '''            Ok(CleanupReceipt::complete(lease, terminated_at_epoch_seconds))\n''',
    '''            Ok(CleanupReceipt::complete(\n                lease.sandbox_id(),\n                lease,\n                terminated_at_epoch_seconds,\n            ))\n''',
)

replace_once(
    "docs/product-technical-gap-baseline.md",
    '''generated `qsr-app-*` remains correlation metadata; cleanup authority is crate-private and non-serializable; forged/deserialized public lease evidence cannot recreate destructive authority; all five live capability columns and single-record network evidence remain fail-closed.''',
    '''generated `qsr-app-*` remains correlation metadata; cleanup authority is crate-private and non-serializable; forged/deserialized public lease evidence cannot recreate destructive authority; cleanup receipts identify the exact container actually removed while `network_id` remains the versioned lease correlation identifier until canonical #48 acquires and binds the Podman network `.ID`; all five live capability columns and single-record network evidence remain fail-closed.''',
)
