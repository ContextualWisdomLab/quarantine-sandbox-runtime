//! Admission regressions for the runtime gate's image-independent ELF loading boundary.
//! Fixtures are data-only for these tests; staging must not execute their bytes.

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux {
    use std::fs;

    use quarantine_sandbox_runtime::{RuntimeGateArtifact, RuntimeGateArtifactError};
    use sha2::{Digest, Sha256};

    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const FILE_BYTES: usize = 512;
    const INTERPRETER_OFFSET: usize = 384;
    const INTERPRETER_PATH: &[u8] = b"/qsr-fixture/image-loader\0";
    const EXIT_X86_64: &[u8] = &[0xb8, 60, 0, 0, 0, 0xbf, 77, 0, 0, 0, 0x0f, 0x05];
    const EXIT_AARCH64: &[u8] = &[
        0xa8, 0x0b, 0x80, 0xd2, 0xa0, 0x09, 0x80, 0xd2, 1, 0, 0, 0xd4,
    ];

    fn elf_fixture(has_interpreter: bool, position_independent: bool) -> Vec<u8> {
        let (machine, exit_code): (u16, &[u8]) = match std::env::consts::ARCH {
            "x86_64" => (62, EXIT_X86_64),
            "aarch64" => (183, EXIT_AARCH64),
            other => panic!("gate ELF fixture does not support architecture {other}"),
        };
        let mut bytes = vec![0_u8; FILE_BYTES];
        bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        let elf_type = 2_u16 + u16::from(position_independent);
        let load_address = u64::from(!position_independent) * 0x400000;
        let program_count = 1_u16 + u16::from(has_interpreter);
        bytes[16..18].copy_from_slice(&elf_type.to_le_bytes());
        bytes[18..20].copy_from_slice(&machine.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&(load_address + 256).to_le_bytes());
        bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_le_bytes());
        bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_le_bytes());
        bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_le_bytes());
        bytes[56..58].copy_from_slice(&program_count.to_le_bytes());

        // ELF requires PT_INTERP to precede loadable segments. The loader path is
        // outside the authenticated gate bytes, even when the gate digest matches.
        if has_interpreter {
            let header = ELF_HEADER_BYTES;
            bytes[header..header + 4].copy_from_slice(&3_u32.to_le_bytes());
            bytes[header + 4..header + 8].copy_from_slice(&4_u32.to_le_bytes());
            bytes[header + 8..header + 16]
                .copy_from_slice(&(INTERPRETER_OFFSET as u64).to_le_bytes());
            bytes[header + 32..header + 40]
                .copy_from_slice(&(INTERPRETER_PATH.len() as u64).to_le_bytes());
            bytes[header + 40..header + 48]
                .copy_from_slice(&(INTERPRETER_PATH.len() as u64).to_le_bytes());
            bytes[header + 48..header + 56].copy_from_slice(&1_u64.to_le_bytes());
            bytes[INTERPRETER_OFFSET..INTERPRETER_OFFSET + INTERPRETER_PATH.len()]
                .copy_from_slice(INTERPRETER_PATH);
        }

        let load_header = ELF_HEADER_BYTES + usize::from(has_interpreter) * PROGRAM_HEADER_BYTES;
        bytes[load_header..load_header + 4].copy_from_slice(&1_u32.to_le_bytes());
        bytes[load_header + 4..load_header + 8].copy_from_slice(&5_u32.to_le_bytes());
        bytes[load_header + 16..load_header + 24].copy_from_slice(&load_address.to_le_bytes());
        bytes[load_header + 32..load_header + 40]
            .copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
        bytes[load_header + 40..load_header + 48]
            .copy_from_slice(&(FILE_BYTES as u64).to_le_bytes());
        bytes[load_header + 48..load_header + 56].copy_from_slice(&4096_u64.to_le_bytes());
        bytes[256..256 + exit_code.len()].copy_from_slice(exit_code);
        bytes
    }

    fn big_endian_elf_fixture() -> Vec<u8> {
        let machine = match std::env::consts::ARCH {
            "x86_64" => 62_u16,
            "aarch64" => 183_u16,
            other => panic!("gate ELF fixture does not support architecture {other}"),
        };
        let mut bytes = elf_fixture(false, false);
        bytes[5] = 2;
        bytes[16..18].copy_from_slice(&2_u16.to_be_bytes());
        bytes[18..20].copy_from_slice(&machine.to_be_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_be_bytes());
        bytes[24..32].copy_from_slice(&0x400100_u64.to_be_bytes());
        bytes[32..40].copy_from_slice(&(ELF_HEADER_BYTES as u64).to_be_bytes());
        bytes[52..54].copy_from_slice(&(ELF_HEADER_BYTES as u16).to_be_bytes());
        bytes[54..56].copy_from_slice(&(PROGRAM_HEADER_BYTES as u16).to_be_bytes());
        bytes[56..58].copy_from_slice(&1_u16.to_be_bytes());
        bytes[ELF_HEADER_BYTES..ELF_HEADER_BYTES + 4].copy_from_slice(&1_u32.to_be_bytes());
        bytes
    }

    fn stage(bytes: &[u8]) -> Result<RuntimeGateArtifact, RuntimeGateArtifactError> {
        let directory = tempfile::tempdir().expect("gate fixture directory must exist");
        let source = directory.path().join("gate-fixture");
        fs::write(&source, bytes).expect("gate fixture must be writable");
        let digest = format!("{:x}", Sha256::digest(bytes));
        RuntimeGateArtifact::stage(&source, &digest, std::env::consts::ARCH)
    }

    fn admits(bytes: &[u8]) -> bool {
        stage(bytes).is_ok()
    }

    #[test]
    fn matching_digest_self_contained_executable_remains_admissible() {
        assert!(admits(&elf_fixture(false, false)));
    }

    #[test]
    fn matching_digest_self_contained_static_pie_remains_admissible() {
        assert!(admits(&elf_fixture(false, true)));
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn host_incompatible_big_endian_executable_is_rejected() {
        assert!(matches!(
            stage(&big_endian_elf_fixture()),
            Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary)
        ));
    }

    #[test]
    fn matching_digest_must_not_authorize_an_image_owned_elf_interpreter() {
        assert!(
            !admits(&elf_fixture(true, false)),
            "a matching gate digest must not authorize PT_INTERP code from the workload image"
        );
    }

    #[test]
    fn elf32_class_cannot_enter_the_elf64_gate_boundary() {
        let mut bytes = elf_fixture(false, false);
        bytes[4] = 1;
        assert!(!admits(&bytes));
    }

    #[test]
    fn unsupported_elf_data_encoding_is_a_loading_boundary_failure() {
        let mut bytes = elf_fixture(false, false);
        bytes[5] = 0;
        assert!(matches!(
            stage(&bytes),
            Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary)
        ));
    }

    #[test]
    fn invalid_elf_ident_version_cannot_enter_the_gate_boundary() {
        let mut bytes = elf_fixture(false, false);
        bytes[6] = 0;
        assert!(!admits(&bytes));
    }

    #[test]
    fn unsupported_elf_object_type_cannot_enter_the_gate_boundary() {
        let mut bytes = elf_fixture(false, false);
        bytes[16..18].copy_from_slice(&1_u16.to_le_bytes());
        assert!(!admits(&bytes));
    }

    #[test]
    fn malformed_elf_header_sizes_cannot_enter_the_gate_boundary() {
        let mut header_size = elf_fixture(false, false);
        header_size[52..54].copy_from_slice(&63_u16.to_le_bytes());
        assert!(!admits(&header_size));

        let mut program_header_size = elf_fixture(false, false);
        program_header_size[54..56].copy_from_slice(&55_u16.to_le_bytes());
        assert!(!admits(&program_header_size));
    }

    #[test]
    fn zero_program_headers_cannot_prove_image_independent_loading() {
        let mut bytes = elf_fixture(false, false);
        bytes[56..58].copy_from_slice(&0_u16.to_le_bytes());
        assert!(!admits(&bytes));
    }

    #[test]
    fn program_header_table_cannot_overlap_the_elf_header() {
        let mut bytes = elf_fixture(false, false);
        bytes[32..40].copy_from_slice(&32_u64.to_le_bytes());
        assert!(!admits(&bytes));
    }

    #[test]
    fn an_elf_without_a_loadable_segment_cannot_be_a_runtime_gate() {
        let mut bytes = elf_fixture(false, false);
        bytes[ELF_HEADER_BYTES..ELF_HEADER_BYTES + 4].copy_from_slice(&0_u32.to_le_bytes());
        assert!(!admits(&bytes));
    }

    #[test]
    fn unrelated_program_header_is_ignored_when_a_loadable_segment_is_present() {
        let mut bytes = elf_fixture(false, false);
        bytes[56..58].copy_from_slice(&2_u16.to_le_bytes());
        let second_header = ELF_HEADER_BYTES + PROGRAM_HEADER_BYTES;
        bytes[second_header..second_header + 4].copy_from_slice(&0x6474_e551_u32.to_le_bytes());
        assert!(admits(&bytes));
    }

    #[test]
    fn truncated_program_header_table_cannot_prove_image_independent_loading() {
        let mut bytes = elf_fixture(true, false);
        bytes.truncate(ELF_HEADER_BYTES + PROGRAM_HEADER_BYTES);
        assert!(
            !admits(&bytes),
            "a truncated ELF program table must fail closed before runtime gate staging"
        );
    }

    #[test]
    fn overflowing_program_header_offset_cannot_prove_image_independent_loading() {
        let mut bytes = elf_fixture(false, false);
        bytes[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(
            !admits(&bytes),
            "unrepresentable ELF table bounds must fail closed rather than use machine identity alone"
        );
    }
}
