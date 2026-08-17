use quarantine_sandbox_runtime::{ArtifactKind, IngestionPolicy, ingest_bytes};

#[test]
fn ingestion_classifies_all_supported_magic_variants_and_text_paths() {
    let fixtures: [(&str, &[u8], ArtifactKind); 11] = [
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
        (
            "fatbe.bin",
            b"\xca\xfe\xba\xbepayload",
            ArtifactKind::MachOExecutable,
        ),
        (
            "fatle.bin",
            b"\xbe\xba\xfe\xcapayload",
            ArtifactKind::MachOExecutable,
        ),
        ("empty.zip", b"PK\x05\x06payload", ArtifactKind::ZipArchive),
        (
            "stream.zip",
            b"PK\x07\x08payload",
            ArtifactKind::ZipArchive,
        ),
        ("script", b"#!/usr/bin/env python3", ArtifactKind::Script),
        ("plain", b"text without extension", ArtifactKind::Text),
        ("unicode.txt", "안전한 텍스트".as_bytes(), ArtifactKind::Text),
        ("invalid.bin", b"\xff\xfe\xfd", ArtifactKind::Unknown),
    ];

    for (name, bytes, expected_kind) in fixtures {
        let artifact = ingest_bytes(name, bytes, &IngestionPolicy::default())
            .expect("bounded fixture must be ingested");
        assert_eq!(artifact.descriptor().artifact_kind, expected_kind);
    }
}

#[test]
fn ingestion_accepts_values_exactly_at_configured_bounds() {
    let policy = IngestionPolicy {
        maximum_artifact_bytes: 3,
        maximum_artifact_name_bytes: 3,
    };
    let artifact = ingest_bytes("abc", b"xyz", &policy)
        .expect("values equal to hard limits must be accepted");
    assert_eq!(artifact.descriptor().artifact_size_bytes, 3);
    assert_eq!(artifact.descriptor().artifact_name, "abc");
}
