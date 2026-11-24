use super::TEST_CWD;
use crate::codex_hooks::constants::{event_names, field_names};

const TRANSCRIPT_PATH: &str = "/workspace/.codex/transcript.jsonl";
const RELATIVE_PATH: &str = "relative/transcript.jsonl";
const NON_NORMAL_PATH: &str = "/workspace/../outside/transcript.jsonl";
const OVERSIZED_PATH_PREFIX: &str = "/";
const PATH_FILL: &str = "a";

#[test]
fn mapping_preserves_authenticated_paths() {
    let identity = super::identity();
    let mut raw = super::wire_test_support::input_value(event_names::SESSION_START);
    raw[field_names::TRANSCRIPT_PATH] = serde_json::json!(TRANSCRIPT_PATH);
    let value = super::wire_test_support::decode_value(raw).unwrap();
    let envelope = crate::codex_hooks::map(&value, &identity).unwrap();
    assert_eq!(
        envelope.metadata.cwd.as_deref(),
        Some(std::path::Path::new(TEST_CWD))
    );
    assert_eq!(
        envelope.metadata.transcript_path.as_deref(),
        Some(std::path::Path::new(TRANSCRIPT_PATH))
    );
}

#[test]
fn metadata_paths_must_be_absolute_and_lexically_normalized() {
    for path in [RELATIVE_PATH, NON_NORMAL_PATH] {
        let mut raw = super::wire_test_support::input_value(event_names::STOP);
        raw[field_names::TRANSCRIPT_PATH] = serde_json::json!(path);
        assert!(
            super::wire_test_support::decode_value(raw).is_err(),
            "{path}"
        );
    }
}

#[test]
fn metadata_paths_are_bounded() {
    let fill = PATH_FILL.repeat(crate::constants::MAX_RUNTIME_PATH_BYTES);
    let mut raw = super::wire_test_support::input_value(event_names::STOP);
    raw[field_names::CWD] = serde_json::json!(format!("{OVERSIZED_PATH_PREFIX}{fill}"));
    assert!(super::wire_test_support::decode_value(raw).is_err());
}
