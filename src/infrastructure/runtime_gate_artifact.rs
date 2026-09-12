//! Immutable host-side staging for the runtime-owned command hold gate.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use sha2::{Digest, Sha256};
use tempfile::TempDir;
use thiserror::Error;

const RUNTIME_GATE_FILE_NAME: &str = "qsr-runtime-gate";
const ELF64_HEADER_BYTES: usize = 64;
const ELF64_PROGRAM_HEADER_BYTES: usize = 56;
const ELF_TYPE_EXECUTABLE: u16 = 2;
const ELF_TYPE_SHARED_OBJECT: u16 = 3;
const ELF_PROGRAM_TYPE_LOAD: u32 = 1;
const ELF_PROGRAM_TYPE_INTERPRETER: u32 = 3;
#[cfg(target_endian = "little")]
const HOST_ELF_DATA_ENCODING: u8 = 1;
#[cfg(target_endian = "big")]
const HOST_ELF_DATA_ENCODING: u8 = 2;

/// Byte order declared by the ELF `EI_DATA` identification field.
#[derive(Clone, Copy)]
enum ElfDataEncoding {
    /// Least-significant byte first.
    LittleEndian,
    /// Most-significant byte first.
    BigEndian,
}

trait RuntimeGateStagingIo {
    fn create_directory(&self) -> io::Result<TempDir>;
    fn open_gate(&self, path: &Path) -> io::Result<File>;
    fn write_gate(&self, staged: &mut File, bytes: &[u8]) -> io::Result<()>;
    fn sync_gate(&self, staged: &File) -> io::Result<()>;
}

struct HostRuntimeGateStagingIo;

impl RuntimeGateStagingIo for HostRuntimeGateStagingIo {
    fn create_directory(&self) -> io::Result<TempDir> {
        tempfile::Builder::new()
            .prefix("qsr-runtime-gate-")
            .tempdir()
    }

    fn open_gate(&self, path: &Path) -> io::Result<File> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o555)
            .open(path)
    }

    fn write_gate(&self, staged: &mut File, bytes: &[u8]) -> io::Result<()> {
        staged.write_all(bytes)
    }

    fn sync_gate(&self, staged: &File) -> io::Result<()> {
        staged.sync_all()
    }
}

/// A digest-bound, architecture-matched runtime gate staged in a private directory.
///
/// Clones share ownership of the same private staging directory, so an adapter can retain the
/// verified bytes without copying or persisting them outside the runtime-owned lifecycle. The
/// staged gate is created read-only and executable so a later container adapter can bind-mount the
/// exact verified bytes without trusting an artifact supplied by the hostile image.
#[derive(Clone, Debug)]
pub struct RuntimeGateArtifact {
    _staging_directory: Arc<TempDir>,
    path: PathBuf,
    sha256: String,
    architecture: String,
}

impl PartialEq for RuntimeGateArtifact {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self._staging_directory, &other._staging_directory)
    }
}

impl Eq for RuntimeGateArtifact {}

impl RuntimeGateArtifact {
    /// Verify and stage a runtime gate from a trusted host release path.
    ///
    /// `expected_sha256` must be a canonical lowercase SHA-256 digest. The expected architecture
    /// must equal the current host architecture and the executable's ELF machine identity, while
    /// the ELF `EI_DATA` encoding must match the compile-time target endianness. The ELF must be a
    /// self-contained ELF64 executable or static PIE with a bounded program-header table, at least
    /// one loadable segment, and no `PT_INTERP` dependency on workload-image code. Symlink and
    /// non-regular-file sources fail closed before bytes are staged.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeGateArtifactError`] when the expected identity is malformed, the source is
    /// not a regular file, the bytes do not match the expected digest or architecture, the ELF
    /// loading boundary is malformed, host-byte-order incompatible, or delegates to an external
    /// interpreter, or private read-only staging cannot be completed.
    pub fn stage(
        source: &Path,
        expected_sha256: &str,
        expected_architecture: &str,
    ) -> Result<Self, RuntimeGateArtifactError> {
        Self::stage_with_staging_io(
            source,
            expected_sha256,
            expected_architecture,
            &HostRuntimeGateStagingIo,
        )
    }

    fn stage_with_staging_io<S: RuntimeGateStagingIo>(
        source: &Path,
        expected_sha256: &str,
        expected_architecture: &str,
        staging_io: &S,
    ) -> Result<Self, RuntimeGateArtifactError> {
        validate_expected_digest(expected_sha256)?;

        let host_architecture = std::env::consts::ARCH;
        if expected_architecture != host_architecture {
            return Err(RuntimeGateArtifactError::ArchitectureMismatch {
                expected: expected_architecture.to_owned(),
                actual: host_architecture.to_owned(),
            });
        }

        let metadata = fs::symlink_metadata(source)
            .map_err(|_| RuntimeGateArtifactError::SourceNotRegularFile)?;
        if !metadata.file_type().is_file() {
            return Err(RuntimeGateArtifactError::SourceNotRegularFile);
        }

        let bytes = fs::read(source).map_err(|_| RuntimeGateArtifactError::SourceReadFailed)?;
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if actual_sha256 != expected_sha256 {
            return Err(RuntimeGateArtifactError::DigestMismatch);
        }

        // Establish the complete executable loading boundary before interpreting
        // machine identity. Malformed ELF class/data/version must therefore be
        // classified as a loading-boundary failure rather than as an architecture
        // mismatch derived from fields that are not yet safe to interpret.
        validate_self_contained_elf_loading(&bytes)?;
        // EI_DATA is part of executable compatibility, not just a parsing hint. The
        // bounded validator can decode either defined byte order, but trusted runtime
        // authority must use the byte order of the target that compiled this adapter.
        if bytes[5] != HOST_ELF_DATA_ENCODING {
            return Err(RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary);
        }
        let actual_architecture = executable_architecture(&bytes);
        if actual_architecture != expected_architecture {
            return Err(RuntimeGateArtifactError::ArchitectureMismatch {
                expected: expected_architecture.to_owned(),
                actual: actual_architecture.to_owned(),
            });
        }

        let staging_directory = staging_io.create_directory().map_err(map_staging_failure)?;
        let path = staging_directory.path().join(RUNTIME_GATE_FILE_NAME);
        let mut staged = staging_io.open_gate(&path).map_err(map_staging_failure)?;
        staging_io
            .write_gate(&mut staged, &bytes)
            .map_err(map_staging_failure)?;
        staging_io.sync_gate(&staged).map_err(map_staging_failure)?;
        drop(staged);

        Ok(Self {
            _staging_directory: Arc::new(staging_directory),
            path,
            sha256: actual_sha256,
            architecture: actual_architecture,
        })
    }

    /// Return the private host path containing the verified gate bytes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Return the verified lowercase SHA-256 digest of the staged gate.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Return the verified executable architecture of the staged gate.
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }
}

/// Failure to establish an immutable runtime gate artifact identity.
#[derive(Debug, Error)]
pub enum RuntimeGateArtifactError {
    /// The configured release digest is not canonical lowercase SHA-256.
    #[error("runtime gate expected digest is not canonical SHA-256")]
    InvalidExpectedDigest,
    /// The configured gate source is absent, a symlink, or not a regular file.
    #[error("runtime gate source is not a regular file")]
    SourceNotRegularFile,
    /// The regular source could not be read after its file type was verified.
    #[error("runtime gate source could not be read")]
    SourceReadFailed,
    /// The source bytes do not match the configured immutable release digest.
    #[error("runtime gate digest does not match the configured release digest")]
    DigestMismatch,
    /// The configured or executable architecture is incompatible with the current runtime host.
    #[error("runtime gate architecture mismatch: expected {expected}, actual {actual}")]
    ArchitectureMismatch {
        /// Required architecture.
        expected: String,
        /// Observed architecture.
        actual: String,
    },
    /// The executable cannot prove a host-compatible, self-contained, bounded ELF loading boundary.
    #[error("runtime gate ELF loading boundary is malformed or uses an external interpreter")]
    UnsafeExecutableLoadingBoundary,
    /// The verified bytes could not be materialized into a private read-only executable staging area.
    #[error("runtime gate private staging failed")]
    StagingFailed,
}

/// Collapse private staging I/O failures into the stable public staging error.
fn map_staging_failure(_: io::Error) -> RuntimeGateArtifactError {
    RuntimeGateArtifactError::StagingFailed
}

/// Require an exact 64-character lowercase hexadecimal SHA-256 release identity.
fn validate_expected_digest(expected_sha256: &str) -> Result<(), RuntimeGateArtifactError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RuntimeGateArtifactError::InvalidExpectedDigest);
    }
    Ok(())
}

/// Decode machine identity from bytes already admitted by the bounded ELF loading validator.
fn executable_architecture(bytes: &[u8]) -> String {
    let machine = match bytes[5] {
        1 => u16::from_le_bytes([bytes[18], bytes[19]]),
        2 => u16::from_be_bytes([bytes[18], bytes[19]]),
        _ => return "elf-unknown-data-encoding".to_owned(),
    };
    match machine {
        62 => "x86_64".to_owned(),
        183 => "aarch64".to_owned(),
        other => format!("elf-machine-{other}"),
    }
}

/// Prove the staged ELF can reach the trusted gate without an image-owned program interpreter.
///
/// The parser accepts only a bounded ELF64 ET_EXEC/ET_DYN program-header table containing at
/// least one `PT_LOAD`. Any malformed table or `PT_INTERP` fails closed before the bytes are
/// admitted to the runtime-owned staging directory.
fn validate_self_contained_elf_loading(bytes: &[u8]) -> Result<(), RuntimeGateArtifactError> {
    let rejected = || RuntimeGateArtifactError::UnsafeExecutableLoadingBoundary;
    if bytes.len() < ELF64_HEADER_BYTES
        || &bytes[..4] != b"\x7fELF"
        || bytes[4] != 2
        || bytes[6] != 1
    {
        return Err(rejected());
    }

    let encoding = match bytes[5] {
        1 => ElfDataEncoding::LittleEndian,
        2 => ElfDataEncoding::BigEndian,
        _ => return Err(rejected()),
    };
    let object_type = read_u16(bytes, 16, encoding);
    if !matches!(object_type, ELF_TYPE_EXECUTABLE | ELF_TYPE_SHARED_OBJECT)
        || read_u32(bytes, 20, encoding) != 1
        || usize::from(read_u16(bytes, 52, encoding)) != ELF64_HEADER_BYTES
        || usize::from(read_u16(bytes, 54, encoding)) != ELF64_PROGRAM_HEADER_BYTES
    {
        return Err(rejected());
    }

    let program_header_count = read_u16(bytes, 56, encoding);
    if program_header_count == 0 {
        return Err(rejected());
    }
    let program_header_offset = read_u64(bytes, 32, encoding);
    if program_header_offset < ELF64_HEADER_BYTES as u64 {
        return Err(rejected());
    }
    // Keep untrusted ELF offsets in their wire-width domain until the complete table is proven to
    // fit the already materialized byte slice. Only then is conversion to host indexing authority
    // safe, so pointer width cannot become a separate acceptance branch.
    let table_bytes = (ELF64_PROGRAM_HEADER_BYTES as u64) * u64::from(program_header_count);
    let table_end = program_header_offset
        .checked_add(table_bytes)
        .ok_or_else(rejected)?;
    if table_end > bytes.len() as u64 {
        return Err(rejected());
    }
    let program_header_offset = program_header_offset as usize;
    let program_header_count = usize::from(program_header_count);

    let mut has_loadable_segment = false;
    for index in 0..program_header_count {
        let header_offset = program_header_offset + index * ELF64_PROGRAM_HEADER_BYTES;
        match read_u32(bytes, header_offset, encoding) {
            ELF_PROGRAM_TYPE_LOAD => has_loadable_segment = true,
            ELF_PROGRAM_TYPE_INTERPRETER => return Err(rejected()),
            _ => {}
        }
    }
    if !has_loadable_segment {
        return Err(rejected());
    }
    Ok(())
}

/// Read one bounded ELF `u16` field using the file-declared byte order.
fn read_u16(bytes: &[u8], offset: usize, encoding: ElfDataEncoding) -> u16 {
    let value = [bytes[offset], bytes[offset + 1]];
    match encoding {
        ElfDataEncoding::LittleEndian => u16::from_le_bytes(value),
        ElfDataEncoding::BigEndian => u16::from_be_bytes(value),
    }
}

/// Read one bounded ELF `u32` field using the file-declared byte order.
fn read_u32(bytes: &[u8], offset: usize, encoding: ElfDataEncoding) -> u32 {
    let value = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    match encoding {
        ElfDataEncoding::LittleEndian => u32::from_le_bytes(value),
        ElfDataEncoding::BigEndian => u32::from_be_bytes(value),
    }
}

/// Read one bounded ELF `u64` field using the file-declared byte order.
fn read_u64(bytes: &[u8], offset: usize, encoding: ElfDataEncoding) -> u64 {
    let value = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
        bytes[offset + 4],
        bytes[offset + 5],
        bytes[offset + 6],
        bytes[offset + 7],
    ];
    match encoding {
        ElfDataEncoding::LittleEndian => u64::from_le_bytes(value),
        ElfDataEncoding::BigEndian => u64::from_be_bytes(value),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File},
        io::{self, Write},
        os::unix::fs::PermissionsExt,
    };

    use sha2::{Digest, Sha256};

    use super::{
        RuntimeGateArtifact, RuntimeGateArtifactError, RuntimeGateStagingIo,
        executable_architecture, validate_expected_digest,
    };

    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const FILE_BYTES: usize = 512;
    #[cfg(target_arch = "aarch64")]
    const HOST_TEST_ELF_MACHINE: u16 = 183;
    #[cfg(target_arch = "x86_64")]
    const HOST_TEST_ELF_MACHINE: u16 = 62;
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    compile_error!("runtime-gate test fixture supports only aarch64 and x86_64 hosts");

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum StagingFailurePoint {
        None,
        Directory,
        Open,
        Write,
        Sync,
    }

    struct ScriptedStagingIo {
        failure: StagingFailurePoint,
    }

    impl RuntimeGateStagingIo for ScriptedStagingIo {
        fn create_directory(&self) -> io::Result<tempfile::TempDir> {
            if self.failure == StagingFailurePoint::Directory {
                return Err(io::Error::other("scripted staging directory failure"));
            }
            tempfile::tempdir()
        }

        fn open_gate(&self, path: &std::path::Path) -> io::Result<File> {
            if self.failure == StagingFailurePoint::Open {
                return Err(io::Error::other("scripted staging open failure"));
            }
            File::create(path)
        }

        fn write_gate(&self, staged: &mut File, bytes: &[u8]) -> io::Result<()> {
            if self.failure == StagingFailurePoint::Write {
                return Err(io::Error::other("scripted staging write failure"));
            }
            staged.write_all(bytes)
        }

        fn sync_gate(&self, staged: &File) -> io::Result<()> {
            if self.failure == StagingFailurePoint::Sync {
                return Err(io::Error::other("scripted staging sync failure"));
            }
            staged.sync_all()
        }
    }

    fn self_contained_gate_bytes() -> Vec<u8> {
        let machine = HOST_TEST_ELF_MACHINE;
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
        bytes
    }

    fn source_fixture() -> (tempfile::TempDir, std::path::PathBuf, Vec<u8>, String) {
        let directory = tempfile::tempdir().expect("runtime-gate fixture directory should exist");
        let source = directory.path().join("self-contained-runtime-gate");
        let bytes = self_contained_gate_bytes();
        fs::write(&source, &bytes).expect("runtime-gate fixture should be writable");
        let expected_sha256 = format!("{:x}", Sha256::digest(&bytes));
        (directory, source, bytes, expected_sha256)
    }

    fn stage_fixture() -> (tempfile::TempDir, RuntimeGateArtifact, Vec<u8>) {
        let (directory, source, bytes, expected_sha256) = source_fixture();
        let artifact =
            RuntimeGateArtifact::stage(&source, &expected_sha256, std::env::consts::ARCH)
                .expect("matching self-contained runtime gate should stage");
        (directory, artifact, bytes)
    }

    #[test]
    fn expected_digest_requires_exact_lowercase_sha256_shape() {
        assert!(validate_expected_digest(&"0".repeat(64)).is_ok());
        let uppercase_error = validate_expected_digest(&"A".repeat(64))
            .expect_err("uppercase digest must fail closed");
        assert_eq!(
            uppercase_error.to_string(),
            "runtime gate expected digest is not canonical SHA-256"
        );
        let short_error =
            validate_expected_digest(&"0".repeat(63)).expect_err("short digest must fail closed");
        assert_eq!(
            short_error.to_string(),
            "runtime gate expected digest is not canonical SHA-256"
        );
    }

    #[test]
    fn staging_failures_are_deterministic_and_fail_closed() {
        let (_directory, source, _bytes, expected_sha256) = source_fixture();
        for failure in [
            StagingFailurePoint::Directory,
            StagingFailurePoint::Open,
            StagingFailurePoint::Write,
            StagingFailurePoint::Sync,
        ] {
            let error = RuntimeGateArtifact::stage_with_staging_io(
                &source,
                &expected_sha256,
                std::env::consts::ARCH,
                &ScriptedStagingIo { failure },
            )
            .expect_err("scripted staging failure must fail closed");
            assert_eq!(error.to_string(), "runtime gate private staging failed");
        }

        let artifact = RuntimeGateArtifact::stage_with_staging_io(
            &source,
            &expected_sha256,
            std::env::consts::ARCH,
            &ScriptedStagingIo {
                failure: StagingFailurePoint::None,
            },
        )
        .expect("scripted staging success must remain available");
        assert_eq!(artifact.sha256(), expected_sha256);
    }

    #[test]
    fn elf_machine_identity_is_endian_aware_after_loading_admission() {
        let mut little_x86_64 = vec![0_u8; 20];
        little_x86_64[..4].copy_from_slice(b"\x7fELF");
        little_x86_64[5] = 1;
        little_x86_64[18..20].copy_from_slice(&62_u16.to_le_bytes());
        assert_eq!(executable_architecture(&little_x86_64), "x86_64");

        let mut big_aarch64 = vec![0_u8; 20];
        big_aarch64[..4].copy_from_slice(b"\x7fELF");
        big_aarch64[5] = 2;
        big_aarch64[18..20].copy_from_slice(&183_u16.to_be_bytes());
        assert_eq!(executable_architecture(&big_aarch64), "aarch64");

        let mut unknown_machine = little_x86_64.clone();
        unknown_machine[18..20].copy_from_slice(&7_u16.to_le_bytes());
        assert_eq!(executable_architecture(&unknown_machine), "elf-machine-7");

        let mut unknown_encoding = little_x86_64;
        unknown_encoding[5] = 0;
        assert_eq!(
            executable_architecture(&unknown_encoding),
            "elf-unknown-data-encoding"
        );
    }

    #[test]
    fn clone_keeps_one_staged_gate_alive_without_copying_identity() {
        let (_directory, artifact, _bytes) = stage_fixture();
        let clone = artifact.clone();
        assert_eq!(artifact, clone);
        drop(artifact);
        assert!(
            clone.path().is_file(),
            "an adapter clone must keep the verified staged gate alive"
        );
    }

    #[test]
    fn staged_gate_permissions_survive_a_remapped_container_user() {
        let (_directory, artifact, _bytes) = stage_fixture();
        let mode = fs::metadata(artifact.path())
            .expect("staged runtime gate metadata should exist")
            .permissions()
            .mode();
        assert_eq!(mode & 0o222, 0, "staged runtime gate must be read-only");
        assert_eq!(
            mode & 0o555,
            0o555,
            "runtime gate must remain readable/executable when Podman maps the container process to a non-owner host identity"
        );
    }
}
