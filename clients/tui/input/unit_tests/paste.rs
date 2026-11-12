use crate::constants::*;
use yolo_hub_terminal_protocol::MAX_INPUT_BYTES;
use yolo_tui_input::{InputError, encode_paste};

#[test]
fn paste_respects_remote_bracket_mode_and_empty_paste_has_no_side_effect() {
    assert_eq!(encode_paste(PASTE_TEXT, true).unwrap(), BRACKETED);
    assert_eq!(
        encode_paste(PASTE_TEXT, false).unwrap(),
        PASTE_TEXT.as_bytes()
    );
    assert!(encode_paste("", false).unwrap().is_empty());
    assert!(encode_paste("", true).unwrap().is_empty());
}

#[test]
fn oversized_or_embedded_control_paste_is_rejected_whole() {
    for text in [PASTE_CONTROL, PASTE_NUL] {
        assert_eq!(encode_paste(text, true), Err(InputError::UnsafePaste));
        assert_eq!(encode_paste(text, false), Err(InputError::UnsafePaste));
    }
    assert_eq!(
        encode_paste(&"a".repeat(MAX_INPUT_BYTES + 1), false),
        Err(InputError::PayloadTooLarge)
    );
    assert_eq!(
        encode_paste(&"a".repeat(MAX_INPUT_BYTES), false)
            .unwrap()
            .len(),
        MAX_INPUT_BYTES
    );
    assert_eq!(
        encode_paste(&"a".repeat(MAX_INPUT_BYTES), true),
        Err(InputError::PayloadTooLarge)
    );
    for error in [
        InputError::UnsafePaste,
        InputError::PayloadTooLarge,
        InputError::UnsupportedKey,
    ] {
        assert!(!error.to_string().is_empty());
    }
}
