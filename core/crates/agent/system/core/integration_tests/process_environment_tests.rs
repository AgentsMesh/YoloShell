use super::*;
use crate::constants;
use crate::provider_fixture as fixture;
use std::io::Cursor;

const CUSTOM_CLAUDE_HOME: &str = "/srv/claude-home";
const CUSTOM_CODEX_HOME: &str = "/srv/codex-home";
const CLAUDE_SETTINGS_IDENTITY: &str = "settings-file-identity";
const DUPLICATE_FIRST_HOME: &str = "/srv/first-home";
const DUPLICATE_SECOND_HOME: &str = "/srv/second-home";
const ENVIRONMENT_FILL: &str = "x";
const IGNORED_KEY: &str = "AUTH_TOKEN";
const IGNORED_VALUE: &str = "do-not-retain";
const INVALID_SELECTED_VALUE: &[u8] = b"/tmp/\xff";
const REMOTE_HOME: &str = "/home/remote";
const SELECTED_KEY_PREFIX: &str = "SELECTED_";
const PTY_CAPABILITY: &str = "pty-capability";

#[test]
fn selected_keys_retain_expected_values_without_unrelated_secrets() {
    let keys = [
        constants::HOME_ENV,
        fixture::PRIMARY_DATA_HOME_ENV,
        fixture::SECONDARY_DATA_HOME_ENV,
        fixture::SETTINGS_IDENTITY_ENV,
        fixture::PTY_CAPABILITY_ENV,
    ];
    let bytes = encoded(&[
        field(constants::HOME_ENV, REMOTE_HOME),
        field(fixture::PRIMARY_DATA_HOME_ENV, CUSTOM_CODEX_HOME),
        field(fixture::SECONDARY_DATA_HOME_ENV, CUSTOM_CLAUDE_HOME),
        field(fixture::SETTINGS_IDENTITY_ENV, CLAUDE_SETTINGS_IDENTITY),
        field(fixture::PTY_CAPABILITY_ENV, PTY_CAPABILITY),
        field(IGNORED_KEY, IGNORED_VALUE),
    ]);
    let selected = parse_selected_environment(&bytes, &keys).unwrap();

    assert_eq!(selected[constants::HOME_ENV], REMOTE_HOME);
    assert_eq!(selected[fixture::PRIMARY_DATA_HOME_ENV], CUSTOM_CODEX_HOME);
    assert_eq!(
        selected[fixture::SECONDARY_DATA_HOME_ENV],
        CUSTOM_CLAUDE_HOME
    );
    assert_eq!(selected[fixture::PTY_CAPABILITY_ENV], PTY_CAPABILITY);
    assert_eq!(
        selected[fixture::SETTINGS_IDENTITY_ENV],
        CLAUDE_SETTINGS_IDENTITY
    );
    assert!(!selected.contains_key(IGNORED_KEY));
}

#[test]
fn raw_field_and_value_budgets_are_independent() {
    let oversized_raw =
        vec![NUL_BYTE; MAX_PROCESS_ENVIRONMENT_RAW_BYTES + PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES];
    assert_error(
        parse_selected_environment(&oversized_raw, &[]),
        process_environment_error::RAW_LIMIT,
    );

    let fields = vec![field(IGNORED_KEY, IGNORED_VALUE); MAX_PROCESS_ENVIRONMENT_FIELDS + 1];
    assert_error(
        parse_selected_environment(&encoded(&fields), &[]),
        process_environment_error::FIELD_LIMIT,
    );

    let selected_key = fixture::PRIMARY_DATA_HOME_ENV;
    let oversized_value = ENVIRONMENT_FILL
        .repeat(MAX_PROCESS_ENVIRONMENT_VALUE_BYTES + PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES);
    assert_error(
        parse_selected_environment(
            &encoded(&[field(selected_key, &oversized_value)]),
            &[selected_key],
        ),
        process_environment_error::VALUE_LIMIT,
    );
}

#[test]
fn unrelated_fields_are_governed_only_by_raw_and_field_budgets() {
    let selected_key = fixture::PRIMARY_DATA_HOME_ENV;
    let long_value = ENVIRONMENT_FILL
        .repeat(MAX_PROCESS_ENVIRONMENT_VALUE_BYTES + PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES);
    let bytes = encoded(&[
        field(IGNORED_KEY, &long_value),
        IGNORED_KEY.as_bytes().to_vec(),
        bytes_field(IGNORED_KEY.as_bytes(), &[u8::MAX]),
        field(selected_key, CUSTOM_CODEX_HOME),
    ]);
    let selected = parse_selected_environment(&bytes, &[selected_key]).unwrap();
    assert_eq!(selected[selected_key], CUSTOM_CODEX_HOME);
}

#[test]
fn selected_byte_and_key_budgets_are_enforced() {
    let selected_key_count = MAX_PROCESS_ENVIRONMENT_BYTES / MAX_PROCESS_ENVIRONMENT_VALUE_BYTES
        + PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES;
    let names: Vec<_> = (0..selected_key_count)
        .map(|index| format!("{SELECTED_KEY_PREFIX}{index}"))
        .collect();
    let value = ENVIRONMENT_FILL.repeat(MAX_PROCESS_ENVIRONMENT_VALUE_BYTES);
    let fields: Vec<_> = names.iter().map(|name| field(name, &value)).collect();
    let keys: Vec<_> = names.iter().map(String::as_str).collect();
    assert_error(
        parse_selected_environment(&encoded(&fields), &keys),
        process_environment_error::SELECTED_LIMIT,
    );

    let too_many = vec![constants::HOME_ENV; MAX_PROCESS_ENVIRONMENT_KEYS + 1];
    assert_error(
        parse_selected_environment(&[], &too_many),
        process_environment_error::KEY_LIMIT,
    );
}

#[test]
fn duplicate_malformed_and_unterminated_inputs_fail_closed() {
    let key = fixture::PRIMARY_DATA_HOME_ENV;
    let duplicate = encoded(&[
        field(key, DUPLICATE_FIRST_HOME),
        field(key, DUPLICATE_SECOND_HOME),
    ]);
    assert_error(
        parse_selected_environment(&duplicate, &[key]),
        process_environment_error::DUPLICATE_KEY,
    );
    let malformed_selected = encoded(&[key.as_bytes().to_vec()]);
    assert_error(
        parse_selected_environment(&malformed_selected, &[key]),
        process_environment_error::MALFORMED,
    );
    assert_error(
        parse_selected_environment(&field(key, CUSTOM_CODEX_HOME), &[key]),
        process_environment_error::UNTERMINATED,
    );
}

#[test]
fn only_selected_non_utf8_values_are_rejected() {
    let key = fixture::PRIMARY_DATA_HOME_ENV;
    let invalid_selected = encoded(&[bytes_field(key.as_bytes(), INVALID_SELECTED_VALUE)]);
    assert_error(
        parse_selected_environment(&invalid_selected, &[key]),
        process_environment_error::VALUE_UTF8,
    );

    let ignored = encoded(&[bytes_field(IGNORED_KEY.as_bytes(), b"\xff")]);
    assert!(
        parse_selected_environment(&ignored, &[key])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn bounded_reader_rejects_content_beyond_the_raw_limit() {
    let bytes =
        vec![NUL_BYTE; MAX_PROCESS_ENVIRONMENT_RAW_BYTES + PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES];
    assert_error(
        read_selected_environment(Cursor::new(bytes), &[]),
        process_environment_error::RAW_LIMIT,
    );
}

fn field(key: &str, value: &str) -> Vec<u8> {
    bytes_field(key.as_bytes(), value.as_bytes())
}

fn bytes_field(key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut field = key.to_vec();
    field.push(ENVIRONMENT_ASSIGNMENT_BYTE);
    field.extend_from_slice(value);
    field
}

fn encoded(fields: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for field in fields {
        bytes.extend_from_slice(field);
        bytes.push(NUL_BYTE);
    }
    bytes
}

fn assert_error<T: std::fmt::Debug>(result: Result<T>, expected: &str) {
    assert_eq!(result.unwrap_err().to_string(), expected);
}
