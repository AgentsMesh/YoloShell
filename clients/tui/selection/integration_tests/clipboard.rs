use crate::fixtures::*;
use base64::{Engine, engine::general_purpose::STANDARD};

#[test]
fn explicit_selection_produces_only_an_encoded_outer_terminal_clipboard_command() {
    let parser = parser(UNICODE_TEXT.as_bytes());
    let selection = selected(&parser, point(FIRST, FIRST), point(FIRST, FOURTH));
    let command = selection.clipboard().unwrap();
    assert!(command.as_bytes().starts_with(CLIPBOARD_PREFIX));
    assert!(command.as_bytes().ends_with(CLIPBOARD_END));
    let encoded =
        &command.as_bytes()[CLIPBOARD_PREFIX.len()..command.as_bytes().len() - CLIPBOARD_END.len()];
    assert_eq!(STANDARD.decode(encoded).unwrap(), "a界e\u{301}".as_bytes());
    assert!(!encoded.contains(&b'\x1b'));
}

#[test]
fn provider_clipboard_commands_never_enter_a_local_selection_or_its_copy_result() {
    let parser = parser(PROVIDER_OSC);
    let selection = selected(&parser, point(FIRST, FIRST), point(FIRST, FOURTH));
    assert_eq!(selection.text().unwrap(), "safe");
    assert_eq!(
        selection.clipboard().unwrap().as_bytes(),
        b"\x1b]52;c;c2FmZQ==\x1b\\"
    );
}
