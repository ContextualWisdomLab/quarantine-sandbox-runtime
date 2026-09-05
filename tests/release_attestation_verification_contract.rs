//! Release-time provenance verification regression.
//!
//! Publication must consume the signed attestations it just produced rather
//! than treating successful attestation creation as proof that the release
//! asset verifies under the repository's trust identity.

use std::{fs, path::Path};

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn release_verifies_provenance_and_sbom_attestations_before_publication() {
    let release = fs::read_to_string(repository_root().join(".github/workflows/release.yml"))
        .expect("release workflow must be readable");
    let publish = release
        .split("  attest-and-release:\n")
        .nth(1)
        .expect("release workflow must expose attest-and-release job");
    let release_create = publish
        .find("gh release create")
        .expect("release workflow must publish through gh release create");
    let before_publication = &publish[..release_create];

    assert!(
        before_publication.contains("gh attestation verify"),
        "release must verify the signed package provenance before publication"
    );
    assert!(
        before_publication.contains("--predicate-type https://spdx.dev/Document/v3"),
        "release must verify the SPDX 3 SBOM attestation for the exact package before publication"
    );
    assert!(
        before_publication.contains("-R \"${GITHUB_REPOSITORY}\"")
            || before_publication.contains("--repo \"${GITHUB_REPOSITORY}\""),
        "attestation verification must bind trust to the owning repository"
    );
}
