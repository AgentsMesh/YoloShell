use super::*;

const LARGE_TAIL_BYTES: usize = 8 * 1024 * 1024;
const ASCII: &str = "x";

#[test]
fn a_full_field_ignores_a_large_paste_without_moving_its_cursor() {
    let mut cursor = Cursor::default();
    let mut text = ASCII.repeat(LIMIT);
    key(&mut cursor, &mut text, KeyCode::Home);
    let value = Event::Paste(ASCII.repeat(LARGE_TAIL_BYTES));
    assert!(!cursor.handle(&mut text, &value, LIMIT));
    assert_eq!(text.len(), LIMIT);
    assert!(text.chars().all(|character| character == 'x'));
    assert_eq!(cursor.position(&text), 0);
}

#[test]
fn only_complete_filtered_graphemes_fit_before_a_large_tail() {
    let tail = ASCII.repeat(LARGE_TAIL_BYTES);
    for (prefix, limit, expected) in [
        ("Ae\u{301}", "Ae".len(), "A"),
        ("Ae\n\u{301}", "Ae".len(), "A"),
        ("A👩‍💻", "A👩".len(), "A"),
        ("A🇺🇸", "A🇺".len(), "A"),
        ("A👩‍💻", "A👩‍💻".len(), "A👩‍💻"),
    ] {
        let mut text = String::new();
        let mut cursor = Cursor::default();
        let paste = Event::Paste(format!("{prefix}{tail}"));
        assert!(cursor.handle(&mut text, &paste, limit));
        assert_eq!(text, expected);
        assert_eq!(cursor.position(&text), text.len());
    }
}
