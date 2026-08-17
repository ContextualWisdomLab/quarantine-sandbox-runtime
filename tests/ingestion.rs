use quarantine_sandbox_runtime::{
    ingest_bytes, ArtifactKind, IngestionError, IngestionPolicy,
};

#[test]
fn ingestion_computes_immutable_sha256_and_preserves_original_bytes() {
    let artifact = ingest_bytes(
        "sample.bin",
        b"abc",
        &IngestionPolicy::default(),
    )
    .expect("valid bytes must be ingested");

    assert_eq!(
        artifact.descriptor().artifact_sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(artifact.descriptor().artifact_size_bytes, 3);
    assert_eq!(artifact.bytes(), b"abc");
}

#[test]
fn ingestion_detects_supported_artifact_families_without_executing_them() {
    let fixtures: [(&str, &[u8], ArtifactKind); 10] = [
        ("sample.exe", b"MZ\x90\x00", ArtifactKind::PortableExecutable),
        ("sample.elf", b"\x7fELF\x02\x01", ArtifactKind::ElfExecutable),
        (
            "sample.macho",
            b"\xcf\xfa\xed\xfe\x07\x00",
            ArtifactKind::MachOExecutable,
        ),
        ("sample.zip", b"PK\x03\x04payload", ArtifactKind::ZipArchive),
        ("sample.pdf", b"%PDF-1.7", ArtifactKind::PdfDocument),
        (
            "sample.doc",
            b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1payload",
            ArtifactKind::OleCompoundDocument,
        ),
        ("sample.sh", b"#!/bin/sh\necho safe", ArtifactKind::Script),
        ("sample.ps1", b"Write-Output 'x'", ArtifactKind::Script),
        ("sample.txt", b"plain UTF-8 text", ArtifactKind::Text),
        ("sample.bin", b"\x00\x01\x02", ArtifactKind::Unknown),
    ];

    for (name, bytes, expected_kind) in fixtures {
        let artifact = ingest_bytes(name, bytes, &IngestionPolicy::default())
            .expect("fixture must be accepted");
        assert_eq!(artifact.descriptor().artifact_kind, expected_kind);
    }
}

#[test]
fn ingestion_rejects_empty_oversized_and_misnamed_artifacts() {
    assert_eq!(
        ingest_bytes("empty.bin", b"", &IngestionPolicy::default()),
        Err(IngestionError::EmptyArtifact)
    );

    let policy = IngestionPolicy {
        maximum_artifact_bytes: 2,
        maximum_artifact_name_bytes: 255,
    };
    assert_eq!(
        ingest_bytes("large.bin", b"abc", &policy),
        Err(IngestionError::ArtifactTooLarge {
            actual_bytes: 3,
            maximum_bytes: 2,
        })
    );

    assert_eq!(
        ingest_bytes("", b"x", &IngestionPolicy::default()),
        Err(IngestionError::EmptyArtifactName)
    );

    assert_eq!(
        ingest_bytes("bad\u{0000}name.bin", b"x", &IngestionPolicy::default()),
        Err(IngestionError::ArtifactNameControlCharacter)
    );

    let name_policy = IngestionPolicy {
        maximum_artifact_bytes: 10,
        maximum_artifact_name_bytes: 3,
    };
    assert_eq!(
        ingest_bytes("long.bin", b"x", &name_policy),
        Err(IngestionError::ArtifactNameTooLong {
            actual_bytes: 8,
            maximum_bytes: 3,
        })
    );
}

#[test]
fn ingestion_policy_must_define_nonzero_bounds() {
    let zero_bytes = IngestionPolicy {
        maximum_artifact_bytes: 0,
        maximum_artifact_name_bytes: 255,
    };
    assert_eq!(
        ingest_bytes("sample.bin", b"x", &zero_bytes),
        Err(IngestionError::InvalidPolicy {
            policy_field: "maximum_artifact_bytes"
        })
    );

    let zero_name = IngestionPolicy {
        maximum_artifact_bytes: 1,
        maximum_artifact_name_bytes: 0,
    };
    assert_eq!(
        ingest_bytes("x", b"x", &zero_name),
        Err(IngestionError::InvalidPolicy {
            policy_field: "maximum_artifact_name_bytes"
        })
    );
}
