use crate::constants::{COLUMNS, ROWS};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_terminal_protocol::MAX_INPUT_BYTES;
use yolo_tui_input::{InputError, encode_key, encode_paste};

const IME_TEXT: &str = "水电费";
const IME_BYTES: &[u8] = b"\xe6\xb0\xb4\xe7\x94\xb5\xe8\xb4\xb9";
const BRACKETED_IME_BYTES: &[u8] = b"\x1b[200~\xe6\xb0\xb4\xe7\x94\xb5\xe8\xb4\xb9\x1b[201~";
const SUPPLEMENTARY_TEXT: &str = "𠮷e\u{301}";
const SUPPLEMENTARY_BYTES: &[u8] = b"\xf0\xa0\xae\xb7e\xcc\x81";

#[test]
fn committed_ime_text_keeps_utf8_bytes_without_meta_prefixes() {
    let parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    for (text, expected) in [
        (IME_TEXT, IME_BYTES),
        (SUPPLEMENTARY_TEXT, SUPPLEMENTARY_BYTES),
    ] {
        let mut bytes = Vec::new();
        for character in text.chars() {
            bytes.extend(
                encode_key(
                    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE),
                    parser.screen(),
                )
                .unwrap(),
            );
        }
        assert_eq!(bytes, expected);
    }
}

#[test]
fn chinese_paste_preserves_content_with_and_without_bracketed_paste() {
    assert_eq!(encode_paste(IME_TEXT, false).unwrap(), IME_BYTES);
    assert_eq!(encode_paste(IME_TEXT, true).unwrap(), BRACKETED_IME_BYTES);
}

#[test]
fn unicode_paste_limit_counts_utf8_bytes_and_bracket_overhead() {
    let mut text = IME_TEXT.repeat(MAX_INPUT_BYTES / IME_BYTES.len());
    text.extend(std::iter::repeat_n(' ', MAX_INPUT_BYTES - text.len()));
    assert_eq!(encode_paste(&text, false).unwrap().len(), MAX_INPUT_BYTES);
    assert_eq!(encode_paste(&text, true), Err(InputError::PayloadTooLarge));
    text.push_str(IME_TEXT);
    assert!(text.chars().count() < MAX_INPUT_BYTES);
    assert_eq!(encode_paste(&text, false), Err(InputError::PayloadTooLarge));
}
