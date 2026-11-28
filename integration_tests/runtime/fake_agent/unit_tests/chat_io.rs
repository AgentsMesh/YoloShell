use super::{constants::*, parser::Editor};

#[test]
fn paste_delimiters_split_across_reads_preserve_unicode_and_native_draft_until_one_submit() {
    const EXISTING: &[u8] = b"native ";
    const APPEND: &str = "中文\r\nsecond line";
    let mut editor = Editor::default();
    assert!(editor.feed(EXISTING).unwrap().is_empty());
    let mut input = PASTE_START.to_vec();
    input.extend_from_slice(APPEND.as_bytes());
    input.extend_from_slice(PASTE_END);
    let mut submitted = Vec::new();
    for byte in input {
        submitted.extend(editor.feed(&[byte]).unwrap());
    }
    assert!(submitted.is_empty());
    assert_eq!(editor.draft(), [EXISTING, APPEND.as_bytes()].concat());
    assert_eq!(
        editor.feed(b"\r").unwrap(),
        vec![[EXISTING, APPEND.as_bytes()].concat()]
    );
    assert!(editor.draft().is_empty());
}

#[test]
fn plain_escape_prefixes_are_data_and_input_budget_is_bounded() {
    let mut editor = Editor::default();
    assert!(editor.feed(b"\x1b[x").unwrap().is_empty());
    assert_eq!(editor.feed(b"\r").unwrap(), vec![b"\x1b[x".to_vec()]);
    assert!(
        editor
            .feed(&vec![b'x'; MAX_INPUT_BYTES])
            .unwrap()
            .is_empty()
    );
    assert_eq!(editor.feed(b"x"), Err(INPUT_LIMIT));
}
