#!/usr/bin/env python3
"""Guarded follow-up for the generated bounded-context error repair.

The broad command-helper migration deliberately rewrites the command receipt region to Core errors.
The application-service runtime-identity helper is adjacent to that command helper in `podman.rs`;
this one-shot pass reasserts its Supporting-context error type and normalizes explicit launch returns
through the Core→Supporting ACL before compilation.
"""

from pathlib import Path


def replace_between(path: str, start: str, end: str, old: str, new: str, minimum: int = 1) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.find(start)
    if start_index < 0:
        raise SystemExit(f"{path}: start marker not found: {start}")
    end_index = text.find(end, start_index + len(start))
    if end_index < 0:
        raise SystemExit(f"{path}: end marker not found: {end}")
    segment = text[start_index:end_index]
    count = segment.count(old)
    if count < minimum:
        raise SystemExit(
            f"{path}: expected at least {minimum} matches between markers, found {count}: {old}"
        )
    segment = segment.replace(old, new)
    target.write_text(text[:start_index] + segment + text[end_index:])


replace_between(
    "src/infrastructure/podman.rs",
    "/// Obtain a fresh unpredictable application-service identity",
    "fn runtime_epoch_seconds(",
    "SandboxRuntimeError",
    "ApplicationServiceError",
    1,
)

replace_between(
    "src/infrastructure/podman.rs",
    "    pub fn launch_at(",
    "    /// Stop a leased service",
    "return Err(error);",
    "return Err(error.into());",
    1,
)
