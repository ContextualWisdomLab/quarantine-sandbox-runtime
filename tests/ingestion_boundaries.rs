//! Boundary tests for artifact formats and ingestion limits.

use quarantine_sandbox_runtime::{ArtifactKind, IngestionPolicy, ingest_bytes};

#[test]
fn ingestion_classifies_supported_thin_magic_variants_and_text_paths() {
    let fixtures: [(&str, &[u8], ArtifactKind); 9] = [
        (
            "mach32be.bin",
            b"\xfe\xed\xfa\xcepayload",
            ArtifactKind::MachOExecutable,
        ),
        (
            "mach32le.bin",
            b"\xce\xfa\xed\xfepayload",
            ArtifactKind::MachOExecutable,
        ),
        (
            "mach64be.bin",
            b"\xfe\xed\xfa\xcfpayload",
            ArtifactKind::MachOExecutable,
        ),
        ("empty.zip", b"PK\x05\x06payload", ArtifactKind::ZipArchive),
        ("stream.zip", b"PK\x07\x08payload", ArtifactKind::ZipArchive),
        ("script", b"#!/usr/bin/env python3", ArtifactKind::Script),
        ("plain", b"text without extension", ArtifactKind::Text),
        (
            "unicode.txt",
            "안전한 텍스트".as_bytes(),
            ArtifactKind::Text,
        ),
        ("invalid.bin", b"\xff\xfe\xfd", ArtifactKind::Unknown),
    ];

    for (name, bytes, expected_kind) in fixtures {
        let artifact = ingest_bytes(name, bytes, &IngestionPolicy::default())
            .expect("bounded fixture must be ingested");
        assert_eq!(artifact.descriptor().artifact_kind, expected_kind);
    }
}

#[test]
fn ingestion_validates_mach_o_fat_headers_and_rejects_java_ambiguity() {
    let mut big_endian_32 = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 1];
    big_endian_32.resize(28, 0);
    let mut little_endian_32 = vec![0xbe, 0xba, 0xfe, 0xca, 1, 0, 0, 0];
    little_endian_32.resize(28, 0);
    let mut big_endian_64 = vec![0xca, 0xfe, 0xba, 0xbf, 0, 0, 0, 1];
    big_endian_64.resize(40, 0);
    let mut little_endian_64 = vec![0xbf, 0xba, 0xfe, 0xca, 1, 0, 0, 0];
    little_endian_64.resize(40, 0);

    for bytes in [
        big_endian_32,
        little_endian_32,
        big_endian_64,
        little_endian_64,
    ] {
        let artifact = ingest_bytes("universal.bin", &bytes, &IngestionPolicy::default())
            .expect("valid universal Mach-O header must be accepted");
        assert_eq!(
            artifact.descriptor().artifact_kind,
            ArtifactKind::MachOExecutable
        );
    }

    let ambiguous_or_invalid = [
        vec![0xca, 0xfe, 0xba, 0xbe],
        vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 0],
        vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 33],
        vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 1],
        {
            let mut java_class = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 61];
            java_class.resize(2_000, 0);
            java_class
        },
    ];

    for bytes in ambiguous_or_invalid {
        let artifact = ingest_bytes("ambiguous.bin", &bytes, &IngestionPolicy::default())
            .expect("bounded ambiguous fixture must still be ingested");
        assert_eq!(artifact.descriptor().artifact_kind, ArtifactKind::Unknown);
    }
}

#[test]
fn ingestion_accepts_values_exactly_at_configured_bounds() {
    let policy = IngestionPolicy {
        maximum_artifact_bytes: 3,
        maximum_artifact_name_bytes: 3,
    };
    let artifact =
        ingest_bytes("abc", b"xyz", &policy).expect("values equal to hard limits must be accepted");
    assert_eq!(artifact.descriptor().artifact_size_bytes, 3);
    assert_eq!(artifact.descriptor().artifact_name, "abc");
}
