//! Executable conformance for the required CWL contract vocabulary 1.0.0.
//!
//! A schema consumer needs an implementation of every required keyword, not
//! only prose and vectors. This witness evaluates every published vector with
//! the crate's own vocabulary evaluator, requires fail-closed refusal of
//! unsupported keyword occurrences, and binds the runtime byte bound to the
//! same canonical serialization the schema keyword uses.

use std::{fs, path::Path};

use quarantine_sandbox_runtime::{
    AnalysisProfile, AnalysisRequest, BoundedSourceContext, CWL_CONTRACT_VOCABULARY_ID,
    ContractError, ContractVocabularyError, CwlContractKeyword,
    canonical_bounded_source_context_json,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CONFORMANCE_VECTORS_PATH: &str =
    "tests/fixtures/cwl_artifact_analysis_contract_vocabulary_1_0_0_vectors.json";
const ANALYSIS_REQUEST_SCHEMA_PATH: &str = "schemas/analysis-request.schema.json";
const PROFILE: &str = "utc_z_only_with_gregorian_day_validation_no_leap_second_notation";

/// SHA-256 of the ten cases published before the pre-publication amendment,
/// re-serialized from the parsed fixture as compact sorted-key JSON (this
/// assumes serde_json's `preserve_order` feature stays off; enabling it fails
/// the test loudly). File whitespace is not pinned. Amendments may only append.
const ORIGINAL_TEN_CASES_SHA256: &str =
    "95dc432de5f0d2d1e5c39ec36ec268cf2f6be7ccafef162812a99343134f2349";

const PUBLISHED_CASE_IDS: [&str; 25] = [
    "max_utf8_bytes_multibyte_boundary",
    "max_utf8_bytes_multibyte_overflow",
    "max_serialized_utf8_bytes_normalized_boundary",
    "max_serialized_utf8_bytes_missing_nullable_normalization",
    "max_serialized_utf8_bytes_jcs_escaping_boundary",
    "rfc3339_profile_gregorian_leap_day_valid",
    "rfc3339_profile_non_leap_february_29_invalid",
    "rfc3339_profile_impossible_month_day_invalid",
    "rfc3339_profile_leap_second_invalid",
    "rfc3339_profile_offset_instead_of_z_invalid",
    "rfc3339_profile_century_non_400_february_29_invalid",
    "rfc3339_profile_century_400_february_29_valid",
    "rfc3339_profile_thirty_day_month_31_invalid",
    "rfc3339_profile_lowercase_z_invalid",
    "rfc3339_profile_fractional_second_valid",
    "rfc3339_profile_lowercase_t_invalid",
    "max_utf8_bytes_non_string_not_applicable",
    "max_serialized_utf8_bytes_unknown_member_invalid",
    "max_serialized_utf8_bytes_non_string_member_invalid",
    "max_serialized_utf8_bytes_non_object_not_applicable",
    "rfc3339_profile_null_not_applicable",
    "rfc3339_profile_century_2100_february_29_invalid",
    "rfc3339_profile_empty_fraction_invalid",
    "rfc3339_profile_fraction_has_no_length_bound_valid",
    "max_utf8_bytes_integer_valued_number_ceiling_valid",
];

const PUBLISHED_REFUSAL_IDS: [&str; 7] = [
    "unknown_cwl_keyword_refused",
    "unknown_rfc3339_profile_refused",
    "rfc3339_profile_value_case_sensitive_refused",
    "max_utf8_bytes_negative_refused",
    "max_utf8_bytes_fractional_refused",
    "max_serialized_utf8_bytes_string_refused",
    "max_utf8_bytes_beyond_exact_json_integer_refused",
];

fn ids(cases: &Value) -> Vec<&str> {
    cases
        .as_array()
        .expect("case array")
        .iter()
        .map(|case| case["id"].as_str().expect("case id"))
        .collect()
}

#[test]
fn published_vector_set_is_exact_and_only_appended() {
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    assert_eq!(ids(&vectors["cases"]), PUBLISHED_CASE_IDS);
    assert_eq!(ids(&vectors["schema_refusal_cases"]), PUBLISHED_REFUSAL_IDS);
    let original = serde_json::to_string(&vectors["cases"].as_array().expect("cases")[..10])
        .expect("serialize original cases");
    assert_eq!(
        format!("{:x}", Sha256::digest(original.as_bytes())),
        ORIGINAL_TEN_CASES_SHA256,
        "the ten originally published cases must stay semantically unchanged"
    );
}

#[test]
fn rfc3339_profile_has_no_length_bound_of_its_own() {
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    let case = vectors["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["id"] == "rfc3339_profile_fraction_has_no_length_bound_valid")
        .expect("published long-fraction vector");
    let instance = case["instance"].as_str().expect("instance string");
    assert!(
        instance.len() > 30,
        "the vector must exceed the request field's sibling 30-byte ceiling"
    );
    assert_eq!(case["valid"], true);
    let field_ceiling = keyword("x-cwl-maxUtf8Bytes", json!(30));
    assert!(
        !field_ceiling.evaluate(&json!(instance)),
        "the request still rejects it, through the sibling byte keyword only"
    );
}

fn read_json(path: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    serde_json::from_str(&fs::read_to_string(path).expect("published artifact must be readable"))
        .expect("published artifact must be valid JSON")
}

fn keyword(name: &str, value: Value) -> CwlContractKeyword {
    CwlContractKeyword::from_schema_keyword(name, &value).expect("supported keyword")
}

#[test]
fn every_published_vector_matches_the_executable_evaluator() {
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    assert_eq!(
        vectors["vocabulary_id"].as_str(),
        Some(CWL_CONTRACT_VOCABULARY_ID)
    );
    let cases = vectors["cases"].as_array().expect("cases array");
    assert_eq!(
        cases.len(),
        PUBLISHED_CASE_IDS.len(),
        "all published vectors must be evaluated"
    );
    for case in cases {
        let id = case["id"].as_str().expect("vector id");
        let evaluator = CwlContractKeyword::from_schema_keyword(
            case["keyword"].as_str().expect("keyword name"),
            &case["keyword_value"],
        )
        .unwrap_or_else(|error| panic!("{id}: published keyword must be supported: {error}"));
        assert_eq!(
            evaluator.evaluate(&case["instance"]),
            case["valid"].as_bool().expect("valid flag"),
            "{id}: evaluator disagrees with the published conformance vector"
        );
        if let Some(canonical) = case.get("canonical_json") {
            assert_eq!(
                canonical_bounded_source_context_json(&case["instance"]).as_deref(),
                Some(canonical.as_str().expect("canonical_json string")),
                "{id}: canonical serialization must equal the published RFC 8785 form"
            );
        }
    }
}

#[test]
fn every_published_refusal_vector_is_refused() {
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    let refusals = vectors["schema_refusal_cases"]
        .as_array()
        .expect("vocabulary publication must contain schema_refusal_cases");
    assert_eq!(
        refusals.len(),
        PUBLISHED_REFUSAL_IDS.len(),
        "refusal semantics must be machine-readable"
    );
    for case in refusals {
        let id = case["id"].as_str().expect("refusal id");
        assert!(
            CwlContractKeyword::from_schema_keyword(
                case["keyword"].as_str().expect("keyword name"),
                &case["keyword_value"],
            )
            .is_err(),
            "{id}: a consumer must refuse this schema keyword occurrence"
        );
    }
}

#[test]
fn every_cwl_keyword_used_by_the_request_schema_is_executable() {
    fn visit(node: &Value, found: &mut Vec<(String, Value)>) {
        match node {
            Value::Object(map) => {
                for (name, value) in map {
                    if name.starts_with("x-cwl-") {
                        found.push((name.clone(), value.clone()));
                    }
                    visit(value, found);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| visit(item, found)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    visit(&read_json(ANALYSIS_REQUEST_SCHEMA_PATH), &mut found);
    assert!(
        found.len() >= 7,
        "request schema keyword discovery must not be empty"
    );
    for (name, value) in found {
        assert!(
            CwlContractKeyword::from_schema_keyword(&name, &value).is_ok(),
            "request schema uses {name}={value} that the evaluator cannot execute"
        );
    }
}

#[test]
fn unknown_keyword_or_profile_value_is_refused_not_ignored() {
    assert_eq!(
        CwlContractKeyword::from_schema_keyword("x-cwl-unknownKeyword", &json!(1)),
        Err(ContractVocabularyError::UnsupportedKeyword)
    );
    for profile in [
        json!("lenient_rfc3339"),
        json!(PROFILE.to_uppercase()),
        json!(1),
    ] {
        assert_eq!(
            CwlContractKeyword::from_schema_keyword("x-cwl-rfc3339Profile", &profile),
            Err(ContractVocabularyError::UnsupportedKeywordValue)
        );
    }
    for bad in [
        json!(-1),
        json!(1.5),
        json!(-1.0),
        json!(1.0e300),
        json!("128"),
        json!(null),
    ] {
        for name in ["x-cwl-maxUtf8Bytes", "x-cwl-maxSerializedUtf8Bytes"] {
            assert_eq!(
                CwlContractKeyword::from_schema_keyword(name, &bad),
                Err(ContractVocabularyError::UnsupportedKeywordValue),
                "{name}={bad} is not a non-negative integer byte ceiling"
            );
        }
    }
    for beyond_exact_json_integer in [
        json!(9_007_199_254_740_992_u64),
        json!(18_446_744_073_709_551_615_u64),
        json!(9_007_199_254_740_992.0),
    ] {
        assert_eq!(
            CwlContractKeyword::from_schema_keyword(
                "x-cwl-maxUtf8Bytes",
                &beyond_exact_json_integer
            ),
            Err(ContractVocabularyError::UnsupportedKeywordValue),
            "{beyond_exact_json_integer} is not exactly representable by every JSON consumer"
        );
    }
    assert_eq!(
        CwlContractKeyword::from_schema_keyword(
            "x-cwl-maxUtf8Bytes",
            &json!(9_007_199_254_740_991_u64)
        ),
        Ok(CwlContractKeyword::MaxUtf8Bytes(9_007_199_254_740_991))
    );
    assert_eq!(
        CwlContractKeyword::from_schema_keyword("x-cwl-maxUtf8Bytes", &json!(30.0)),
        Ok(CwlContractKeyword::MaxUtf8Bytes(30)),
        "Draft 2020-12 treats an integer-valued number as an integer"
    );
}

#[test]
fn rfc3339_profile_rejects_hostile_lookalikes() {
    let profile = keyword("x-cwl-rfc3339Profile", json!(PROFILE));
    for invalid in [
        "",
        " 2024-02-29T23:59:59Z",
        "2024-02-29T23:59:59Z\n",
        "\u{feff}2024-02-29T23:59:59Z",
        "2024-02-29T23:59:59.Z",
        "2024-02-29T23:59:59+00:00",
        "2024-02-29T23:59:59-00:00",
        "2024-02-29T23:59:59Z+00:00",
        "2024-02-29 23:59:59Z",
        "2024-2-29T23:59:59Z",
        "+2024-02-29T23:59:59Z",
        "20240-02-29T23:59:59Z",
        "２０２４-02-29T23:59:59Z",
        "202é-02-29T00:00:00Z",
        "2024-13-01T00:00:00Z",
        "2024-00-10T00:00:00Z",
        "2024-01-00T00:00:00Z",
        "2024-01-32T00:00:00Z",
        "2024-06-31T00:00:00Z",
        "2024-09-31T00:00:00Z",
        "2024-11-31T00:00:00Z",
        "2100-02-29T00:00:00Z",
        "2024-01-01T24:00:00Z",
        "2024-01-01T23:60:00Z",
    ] {
        assert!(!profile.evaluate(&json!(invalid)), "{invalid:?}");
    }
    for valid in [
        "0000-02-29T00:00:00Z",
        "2400-02-29T00:00:00Z",
        "2024-02-29T23:59:59.1Z",
        "2024-02-29T23:59:59.12345678901234567890Z",
    ] {
        assert!(profile.evaluate(&json!(valid)), "{valid:?}");
    }
}

#[test]
fn keywords_ignore_non_applicable_instance_types_like_draft_2020_12() {
    let string_ceiling = keyword("x-cwl-maxUtf8Bytes", json!(0));
    let profile = keyword("x-cwl-rfc3339Profile", json!(PROFILE));
    let object_ceiling = keyword("x-cwl-maxSerializedUtf8Bytes", json!(0));
    for other in [
        json!(null),
        json!(3),
        json!(true),
        json!([1]),
        json!({"a": 1}),
    ] {
        assert!(string_ceiling.evaluate(&other));
        assert!(profile.evaluate(&other));
    }
    for other in [json!(null), json!("x"), json!(3), json!([1])] {
        assert!(object_ceiling.evaluate(&other));
    }
    assert!(string_ceiling.evaluate(&json!("")));
    assert!(!string_ceiling.evaluate(&json!("a")));
}

#[test]
fn utf8_ceiling_counts_octets_across_multibyte_boundaries() {
    let ceiling = keyword("x-cwl-maxUtf8Bytes", json!(30));
    assert!(ceiling.evaluate(&json!("a".repeat(30))));
    assert!(!ceiling.evaluate(&json!("a".repeat(31))));
    assert!(ceiling.evaluate(&json!(format!("{}é", "a".repeat(28)))));
    assert!(!ceiling.evaluate(&json!(format!("{}é", "a".repeat(29)))));
    assert!(ceiling.evaluate(&json!(format!("{}😀", "a".repeat(26)))));
    assert!(!ceiling.evaluate(&json!(format!("{}😀", "a".repeat(27)))));
}

#[test]
fn serialized_ceiling_rejects_shapes_outside_bounded_source_context() {
    let ceiling = keyword("x-cwl-maxSerializedUtf8Bytes", json!(1024));
    for invalid in [
        json!({"unexpected_field": "x"}),
        json!({"original_file_name": 7}),
        json!({"original_file_name": {"nested": "x"}}),
        json!({"submitted_at": ["2024-02-29T23:59:59Z"]}),
    ] {
        assert!(!ceiling.evaluate(&invalid), "{invalid}");
        assert_eq!(canonical_bounded_source_context_json(&invalid), None);
    }
    assert_eq!(
        canonical_bounded_source_context_json(&json!("not an object")),
        None
    );
    let all_null = "{\"declared_media_type\":null,\"host_artifact_reference\":null,\"original_file_name\":null,\"source_channel_code\":null,\"submitted_at\":null}";
    assert_eq!(
        canonical_bounded_source_context_json(&json!({})).as_deref(),
        Some(all_null)
    );
    assert_eq!(
        canonical_bounded_source_context_json(&json!({"submitted_at": null})).as_deref(),
        Some(all_null),
        "an absent member and an explicit null count identically"
    );
    assert_eq!(
        canonical_bounded_source_context_json(
            &json!({"original_file_name": "\u{0}\u{1f}\u{7f}/\u{2028}"})
        )
        .as_deref(),
        Some(
            "{\"declared_media_type\":null,\"host_artifact_reference\":null,\"original_file_name\":\"\\u0000\\u001f\u{7f}/\u{2028}\",\"source_channel_code\":null,\"submitted_at\":null}"
        ),
        "RFC 8785 escapes only quote, backslash and C0 controls, using lower-case hex"
    );
}

#[test]
fn canonical_count_equals_the_previous_serde_count_for_sparse_contexts() {
    // The runtime previously counted `serde_json::to_vec(context)`. Without
    // `skip_serializing_if`, serde already wrote every absent member as null,
    // and its escaping matches RFC 8785, so moving to the canonical form must
    // not change the count for any context, sparse or full.
    for context in [
        BoundedSourceContext {
            source_channel_code: None,
            original_file_name: Some(format!("{}\u{1}é😀", "\"".repeat(200))),
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: None,
        },
        BoundedSourceContext {
            source_channel_code: Some("direct_api".to_owned()),
            original_file_name: None,
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: Some("2024-02-29T23:59:59Z".to_owned()),
        },
        context_with_file_name(format!("{}a", "\"".repeat(106))),
    ] {
        assert_eq!(
            context.canonical_json().len(),
            serde_json::to_vec(&context).expect("serde").len()
        );
    }
}

/// Hand evaluation of the request schema's `submitted_at` pattern
/// `^[0-9]{4}-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]+)?Z$`.
fn matches_schema_submitted_at_pattern(value: &str) -> bool {
    let b = value.as_bytes();
    let d = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    let between = |i: usize, low: u8, high: u8| b.get(i).is_some_and(|c| (low..=high).contains(c));
    if b.len() < 20 || !(0..4).all(d) || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return false;
    }
    let month =
        (b[5] == b'0' && between(6, b'1', b'9')) || (b[5] == b'1' && between(6, b'0', b'2'));
    let day = (between(8, b'0', b'2') && d(9)) || (b[8] == b'3' && between(9, b'0', b'1'));
    let hour = (between(11, b'0', b'1') && d(12)) || (b[11] == b'2' && between(12, b'0', b'3'));
    let rest = &b[19..];
    let fraction = rest == b"Z"
        || (rest.len() > 2
            && rest[0] == b'.'
            && rest[rest.len() - 1] == b'Z'
            && rest[1..rest.len() - 1].iter().all(u8::is_ascii_digit));
    month
        && day
        && hour
        && b[13] == b':'
        && between(14, b'0', b'5')
        && d(15)
        && b[16] == b':'
        && between(17, b'0', b'5')
        && d(18)
        && fraction
}

#[test]
fn schema_pattern_admits_every_profile_valid_vector() {
    let schema = read_json(ANALYSIS_REQUEST_SCHEMA_PATH);
    assert_eq!(
        schema["properties"]["bounded_source_context"]["properties"]["submitted_at"]["pattern"],
        r"^[0-9]{4}-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9](?:\.[0-9]+)?Z$",
        "the hand evaluation below is bound to this exact pattern"
    );
    let profile = keyword("x-cwl-rfc3339Profile", json!(PROFILE));
    let vectors = read_json(CONFORMANCE_VECTORS_PATH);
    let mut checked = 0;
    for case in vectors["cases"].as_array().expect("cases") {
        if let Some(instance) = case["instance"].as_str()
            && case["keyword"] == "x-cwl-rfc3339Profile"
            && profile.evaluate(&case["instance"])
        {
            assert!(
                matches_schema_submitted_at_pattern(instance),
                "{instance}: the structural pattern must never be stricter than the profile"
            );
            checked += 1;
        }
    }
    assert!(checked >= 4);
    assert!(!matches_schema_submitted_at_pattern("2024-02-29t23:59:59Z"));
    assert!(!matches_schema_submitted_at_pattern(
        "2024-02-29T23:59:59.Z"
    ));
}

fn context_with_file_name(original_file_name: String) -> BoundedSourceContext {
    BoundedSourceContext {
        source_channel_code: Some("a".repeat(64)),
        original_file_name: Some(original_file_name),
        declared_media_type: Some(format!("{}/{}", "a".repeat(127), "b".repeat(127))),
        host_artifact_reference: Some("h".repeat(128)),
        submitted_at: Some("2024-02-29T23:59:59.123456789Z".to_owned()),
    }
}

fn request(context: BoundedSourceContext) -> AnalysisRequest {
    AnalysisRequest {
        schema_version: "1.0.0".to_owned(),
        request_id: "serialized-parity".to_owned(),
        profile: AnalysisProfile::StaticOnly,
        bounded_source_context: Some(context),
    }
}

#[test]
fn runtime_and_schema_count_the_same_serialized_bytes_at_the_boundary() {
    let schema = read_json(ANALYSIS_REQUEST_SCHEMA_PATH);
    let schema_ceiling =
        &schema["properties"]["bounded_source_context"]["x-cwl-maxSerializedUtf8Bytes"];
    let ceiling = keyword("x-cwl-maxSerializedUtf8Bytes", schema_ceiling.clone());
    assert_eq!(ceiling, CwlContractKeyword::MaxSerializedUtf8Bytes(1024));

    // Every other field sits at its own ceiling; 212 quotes (two canonical
    // bytes each) plus one `a` bring the canonical context to exactly 1024.
    let at_limit = format!("{}a", "\"".repeat(212));
    for (file_name, expected_valid) in [(at_limit.clone(), true), (format!("{at_limit}a"), false)] {
        let context = context_with_file_name(file_name);
        let instance = serde_json::to_value(&context).expect("context json");
        let canonical_length = canonical_bounded_source_context_json(&instance)
            .expect("canonical context")
            .len();
        assert_eq!(canonical_length, if expected_valid { 1024 } else { 1025 });
        assert_eq!(ceiling.evaluate(&instance), expected_valid);
        let runtime = request(context).validate();
        if expected_valid {
            assert_eq!(
                runtime,
                Ok(()),
                "runtime must accept the schema's inclusive boundary"
            );
        } else {
            assert_eq!(
                runtime,
                Err(ContractError::BoundedSourceContextTooLarge {
                    maximum_bytes: 1024
                }),
                "runtime must reject one byte past the schema boundary"
            );
        }
    }
}

#[test]
fn runtime_submitted_at_matches_the_vocabulary_profile_and_byte_ceiling() {
    let profile = keyword("x-cwl-rfc3339Profile", json!(PROFILE));
    let ceiling = keyword("x-cwl-maxUtf8Bytes", json!(30));
    for candidate in [
        "2024-02-29T23:59:59Z",
        "2023-02-29T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2000-02-29T00:00:00Z",
        "2024-02-29T23:59:59.123456789Z",
        "2024-02-29T23:59:59.1234567890Z",
        "2024-02-29t23:59:59Z",
        "2024-02-29T23:59:59z",
        "2016-12-31T23:59:60Z",
    ] {
        let schema_valid =
            profile.evaluate(&json!(candidate)) && ceiling.evaluate(&json!(candidate));
        let context = BoundedSourceContext {
            source_channel_code: None,
            original_file_name: None,
            declared_media_type: None,
            host_artifact_reference: None,
            submitted_at: Some(candidate.to_owned()),
        };
        assert_eq!(
            request(context).validate().is_ok(),
            schema_valid,
            "{candidate}: runtime and published schema keywords must agree"
        );
    }
}
