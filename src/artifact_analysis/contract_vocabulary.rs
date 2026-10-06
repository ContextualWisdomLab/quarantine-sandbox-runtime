//! Executable semantics of the required CWL artifact-analysis contract vocabulary 1.0.0.
//!
//! The request schema declares a dialect that marks the CWL vocabulary as
//! required. A consumer may therefore process that schema only when it can
//! execute every CWL keyword. This module is the producer's reference
//! implementation of those keyword semantics, exercised against the published
//! conformance vectors. [`super::AnalysisRequest::validate`] shares exactly two
//! parts with it: the Gregorian timestamp predicate and the canonical
//! serializer counted by `x-cwl-maxSerializedUtf8Bytes`. The numeric per-field
//! ceilings remain separate runtime constants; tests compare the context and
//! `submitted_at` ceilings with the schema. Structural schema keywords such as
//! `minProperties` are outside this module.
//!
//! Precondition: instances must come from a parser that rejects non-I-JSON
//! (duplicate member names, unpaired surrogates). A [`serde_json::Value`]
//! cannot represent either violation, so this module cannot detect them; the
//! derived `AnalysisRequest` deserializer rejects both.
//!
//! Construction is the only fallible step: an unknown keyword, an unknown
//! profile value, or a malformed keyword value is a schema-level refusal.
//! Instance evaluation always yields a boolean and, like Draft 2020-12
//! type-specific keywords, treats instance types the keyword does not apply to
//! as valid; the sibling `type` keyword owns type rejection.

use serde_json::Value;
use thiserror::Error;

/// Required vocabulary identity whose semantics this module executes.
pub const CWL_CONTRACT_VOCABULARY_ID: &str =
    "https://contextualwisdomlab.org/vocab/quarantine-artifact-analysis-contract-1.0.0";

const MAX_UTF8_BYTES_KEYWORD: &str = "x-cwl-maxUtf8Bytes";
const MAX_SERIALIZED_UTF8_BYTES_KEYWORD: &str = "x-cwl-maxSerializedUtf8Bytes";
const RFC3339_PROFILE_KEYWORD: &str = "x-cwl-rfc3339Profile";
const UTC_GREGORIAN_PROFILE: &str =
    "utc_z_only_with_gregorian_day_validation_no_leap_second_notation";

/// Largest integer a JSON number can carry without IEEE-754 precision loss.
const MAX_EXACT_JSON_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Nullable `BoundedSourceContext` members in RFC 8785 (UTF-16 code unit) order.
///
/// All names are ASCII, so this order also equals UTF-8 byte order.
const BOUNDED_SOURCE_CONTEXT_MEMBERS: [&str; 5] = [
    "declared_media_type",
    "host_artifact_reference",
    "original_file_name",
    "source_channel_code",
    "submitted_at",
];

/// Schema-level refusal of a CWL keyword occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum ContractVocabularyError {
    /// The keyword is in the `x-cwl-` namespace but not defined by vocabulary 1.0.0.
    #[error("unsupported CWL contract vocabulary keyword")]
    UnsupportedKeyword,
    /// The keyword is defined but its value is outside the 1.0.0 definition.
    #[error("unsupported CWL contract vocabulary keyword value")]
    UnsupportedKeywordValue,
}

/// One executable CWL keyword occurrence taken from a schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CwlContractKeyword {
    /// `x-cwl-maxUtf8Bytes`: UTF-8 octet ceiling for a string instance.
    MaxUtf8Bytes(u64),
    /// `x-cwl-maxSerializedUtf8Bytes`: canonical UTF-8 ceiling for a bounded source context.
    MaxSerializedUtf8Bytes(u64),
    /// `x-cwl-rfc3339Profile` with the UTC Gregorian no-leap-second profile.
    UtcGregorianTimestampProfile,
}

impl CwlContractKeyword {
    /// Interpret one keyword/value pair found in a schema.
    ///
    /// # Errors
    ///
    /// Returns [`ContractVocabularyError`] when a consumer must refuse the
    /// schema instead of treating the keyword as an annotation.
    pub fn from_schema_keyword(
        keyword_name: &str,
        keyword_value: &Value,
    ) -> Result<Self, ContractVocabularyError> {
        match keyword_name {
            MAX_UTF8_BYTES_KEYWORD => byte_ceiling(keyword_value).map(Self::MaxUtf8Bytes),
            MAX_SERIALIZED_UTF8_BYTES_KEYWORD => {
                byte_ceiling(keyword_value).map(Self::MaxSerializedUtf8Bytes)
            }
            RFC3339_PROFILE_KEYWORD => {
                if keyword_value.as_str() == Some(UTC_GREGORIAN_PROFILE) {
                    Ok(Self::UtcGregorianTimestampProfile)
                } else {
                    Err(ContractVocabularyError::UnsupportedKeywordValue)
                }
            }
            _ => Err(ContractVocabularyError::UnsupportedKeyword),
        }
    }

    /// Evaluate the keyword assertion against one JSON instance.
    #[must_use]
    pub fn evaluate(&self, instance: &Value) -> bool {
        match (self, instance) {
            (Self::MaxUtf8Bytes(ceiling), Value::String(text)) => fits(text.len(), *ceiling),
            (Self::MaxSerializedUtf8Bytes(ceiling), Value::Object(_)) => {
                canonical_bounded_source_context_json(instance)
                    .is_some_and(|canonical| fits(canonical.len(), *ceiling))
            }
            (Self::UtcGregorianTimestampProfile, Value::String(text)) => {
                is_utc_gregorian_timestamp(text)
            }
            _ => true,
        }
    }
}

/// RFC 8785 canonical JSON of a bounded source context after null materialization.
///
/// Returns `None` when the object has a member outside the closed 1.0.0
/// member set or a member value that is neither a string nor `null`; such an
/// instance is invalid for `x-cwl-maxSerializedUtf8Bytes`. A non-object
/// instance also yields `None`.
#[must_use]
pub fn canonical_bounded_source_context_json(instance: &Value) -> Option<String> {
    let members = instance.as_object()?;
    if members
        .keys()
        .any(|name| !BOUNDED_SOURCE_CONTEXT_MEMBERS.contains(&name.as_str()))
    {
        return None;
    }
    let mut values = [None; 5];
    for (slot, name) in values.iter_mut().zip(BOUNDED_SOURCE_CONTEXT_MEMBERS) {
        *slot = match members.get(name) {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => Some(text.as_str()),
            Some(_) => return None,
        };
    }
    Some(canonical_context_from_members(values))
}

/// Canonical JSON for the five nullable members, given in canonical member order.
pub(super) fn canonical_context_from_members(values: [Option<&str>; 5]) -> String {
    let mut canonical = String::from("{");
    for (index, (name, value)) in BOUNDED_SOURCE_CONTEXT_MEMBERS
        .iter()
        .zip(values)
        .enumerate()
    {
        if index > 0 {
            canonical.push(',');
        }
        push_canonical_string(&mut canonical, name);
        canonical.push(':');
        match value {
            Some(text) => push_canonical_string(&mut canonical, text),
            None => canonical.push_str("null"),
        }
    }
    canonical.push('}');
    canonical
}

/// Append an RFC 8785 §3.2.2.2 serialized JSON string.
fn push_canonical_string(output: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{09}' => output.push_str("\\t"),
            '\u{0A}' => output.push_str("\\n"),
            '\u{0C}' => output.push_str("\\f"),
            '\u{0D}' => output.push_str("\\r"),
            control if u32::from(control) < 0x20 => {
                let code = u32::from(control) as usize;
                output.push_str("\\u00");
                output.push(char::from(HEX[code >> 4]));
                output.push(char::from(HEX[code & 0x0F]));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

/// Whether `value` satisfies the UTC Gregorian no-leap-second profile.
///
/// This is the vocabulary rule only; the request contract additionally bounds
/// the field's UTF-8 length through `x-cwl-maxUtf8Bytes`.
pub(super) fn is_utc_gregorian_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 || bytes.last() != Some(&b'Z') {
        return false;
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        ascii_decimal(&bytes[0..4]),
        ascii_decimal(&bytes[5..7]),
        ascii_decimal(&bytes[8..10]),
        ascii_decimal(&bytes[11..13]),
        ascii_decimal(&bytes[14..16]),
        ascii_decimal(&bytes[17..19]),
    ) else {
        return false;
    };
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return false;
    }
    let fraction = &bytes[19..bytes.len() - 1];
    fraction.is_empty()
        || (fraction.len() > 1
            && fraction[0] == b'.'
            && fraction[1..].iter().all(u8::is_ascii_digit))
}

fn ascii_decimal(digits: &[u8]) -> Option<u32> {
    digits.iter().try_fold(0_u32, |value, digit| {
        digit
            .is_ascii_digit()
            .then(|| value * 10 + u32::from(digit - b'0'))
    })
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if is_gregorian_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

const fn is_gregorian_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn byte_ceiling(keyword_value: &Value) -> Result<u64, ContractVocabularyError> {
    keyword_value
        .as_u64()
        .filter(|ceiling| *ceiling <= MAX_EXACT_JSON_INTEGER as u64)
        .or_else(|| {
            keyword_value
                .as_f64()
                .filter(|number| {
                    number.fract() == 0.0 && (0.0..=MAX_EXACT_JSON_INTEGER).contains(number)
                })
                .map(|number| number as u64)
        })
        .ok_or(ContractVocabularyError::UnsupportedKeywordValue)
}

fn fits(byte_count: usize, ceiling: u64) -> bool {
    u64::try_from(byte_count).is_ok_and(|count| count <= ceiling)
}

#[cfg(test)]
mod tests {
    use super::{ascii_decimal, fits, is_utc_gregorian_timestamp, push_canonical_string};

    #[test]
    fn ascii_decimal_rejects_non_ascii_digit_bytes() {
        assert_eq!(ascii_decimal(b"2024"), Some(2024));
        assert_eq!(ascii_decimal(b"20a4"), None);
        assert_eq!(ascii_decimal("٢".as_bytes()), None);
    }

    #[test]
    fn multibyte_input_at_separator_offsets_is_rejected_without_panicking() {
        assert!(!is_utc_gregorian_timestamp("202é-02-29T00:00:00Z"));
        assert!(!is_utc_gregorian_timestamp("2024-02-2éT0:00:00Z"));
    }

    #[test]
    fn canonical_string_uses_short_escapes_and_lowercase_hex() {
        let mut output = String::new();
        push_canonical_string(&mut output, "\u{8}\t\n\u{c}\r\u{1}\u{1b}\"\\/\u{7f}é");
        assert_eq!(output, "\"\\b\\t\\n\\f\\r\\u0001\\u001b\\\"\\\\/\u{7f}é\"");
    }

    #[test]
    fn ceiling_comparison_is_inclusive() {
        assert!(fits(3, 3));
        assert!(!fits(4, 3));
    }
}
