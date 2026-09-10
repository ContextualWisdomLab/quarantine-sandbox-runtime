//! Shared data-only ELF fixture for runtime-gate staging tests.

use std::{
    fs,
    path::{Path, PathBuf},
};

const ELF_HEADER_BYTES: usize = 64;
const PROGRAM_HEADER_BYTES: usize = 56;
const FILE_BYTES: usize = 512;

/// Write a self-contained ELF64 fixture with one executable `PT_LOAD` segment.
///
/// The bytes are never executed. They exist so staging tests exercise an image-independent
/// executable instead of the dynamically linked Rust test harness, whose `PT_INTERP` is correctly
/// rejected by the production runtime-gate admission boundary.
pub fn write_self_contained_gate(directory: &Path) -> (PathBuf, Vec<u8>) {
    let machine = match std::env::consts::ARCH {
        "x86_64" => 62_u16,
        "aarch64" => 183_u16,
        other => panic!("runtime-gate test fixture does not support architecture {other}"),
    };
    let mut bytes = vec![0_u8; FILE_BYTES];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    bytes[16..18].copy_from_slice(&2_u16.to_le_bytes());
    bytes[18..20].copy_from_slice(&machine.to_le_bytes());
    bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
    bytes[24..32].copy_from_slice(&0x400100_u64.to_le_bytes());
    bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_le_bytes());
    bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_le_bytes());
    bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_le_bytes());
    bytes[56..58].copy_from_slice(&1_u16.to_le_bytes());

    let header = ELF_HEADER_BYTES;
    bytes[header..header + 4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[header + 4..header + 8].copy_from_slice(&5_u32.to_le_bytes());
    bytes[header + 16..header + 24].copy_from_slice(&0x400000_u64.to_le_bytes());
    bytes[header + 32..header + 40].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
    bytes[header + 40..header + 48].copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
    bytes[header + 48..header + 56].copy_from_slice(&4096_u64.to_le_bytes());

    let source = directory.join("self-contained-runtime-gate");
    fs::write(&source, &bytes).expect("self-contained runtime-gate fixture should be writable");
    (source, bytes)
}
