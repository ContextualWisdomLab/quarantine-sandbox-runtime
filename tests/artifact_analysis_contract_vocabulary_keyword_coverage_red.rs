//! Required-vocabulary publication coverage for every declared CWL keyword.
//!
//! A required JSON Schema vocabulary cannot expose a keyword syntactically and
//! defer its semantics to a later mutable publication under the same versioned
//! vocabulary URI. Every custom keyword declared by the dialect must therefore
//! have normative semantics and at least one conformance vector in the same
//! immutable vocabulary version before release.

use std::{fs, path::Path};

use serde_json::Value;

const DIALECT_PATH: &str =
    "schemas/cwl-artifact-analysis-contract-dialect-1.0.0.schema.json";
const VOCABULARY_SPEC_PATH: &str =
    "docs/contracts/cwl_artifact_analysis_contract_vocabulary_1_0_0.md";
const CONFORMANCE_VECTORS_PATH: &str =
    "tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json";
const CWL_CONTRACT_VOCABULARY: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";

fn repository_path(relative_path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn read_text(relative_path: &str) -> String {
    fs::read_to_string(repository_path(relative_path)).unwrap_or_else(|error| {
        panic!("required vocabulary artifact {relative_path} must be readable: {error}")
    })
}

fn read_json(relative_path: &str) -> Value {
    serde_json::from_str(&read_text(relative_path))
        .unwrap_or_else(|error| panic!("required JSON artifact {relative_path} must parse: {error}"))
}

#[test]
fn required_vocabulary_declares_no_undefined_custom_keywords() {
    let dialect = read_json(DIALECT_PATH);
    assert_eq!(
        dialect["$vocabulary"][CWL_CONTRACT_VOCABULARY].as_bool(),
        Some(true),
        "the CWL vocabulary must remain required"
    );

    let declared_keywords = dialect["properties"]
        .as_object()
        .expect("CWL dialect must declare its custom keyword schemas");
    let specification = read_text(VOCABULARY_SPEC_PATH);
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    let cases = vectors["cases"]
        .as_array()
        .expect("CWL vocabulary publication must contain conformance cases");

    for keyword in declared_keywords
        .keys()
        .filter(|keyword| keyword.starts_with("x-cwl-"))
    {
        let normative_heading = format!("## `{keyword}`");
        assert!(
            specification.contains(&normative_heading),
            "required vocabulary keyword {keyword} is declared by dialect 1.0.0 but has no normative semantics in the same 1.0.0 publication"
        );
        assert!(
            cases
                .iter()
                .any(|case| case["keyword"].as_str() == Some(keyword.as_str())),
            "required vocabulary keyword {keyword} is declared by dialect 1.0.0 but has no conformance vector in the same 1.0.0 publication"
        );
    }
}
