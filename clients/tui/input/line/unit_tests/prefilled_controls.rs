use super::*;
use ratatui::text::Span;

const PREFIX: &str = "/tmp/";
const CONTROLS: &[&str] = &["\n", "\r\n", "\t", "\x1b", "\u{85}"];
const SMALL_WIDTHS: std::ops::RangeInclusive<u16> = 0..=16;

#[test]
fn existing_control_bytes_are_preserved_and_editing_uses_raw_grapheme_offsets() {
    for control_value in CONTROLS {
        let mut text = format!("{PREFIX}{control_value}中");
        let mut cursor = Cursor::default();
        assert_eq!(cursor.position(&text), text.len());
        key(&mut cursor, &mut text, KeyCode::Backspace);
        assert_eq!(text, format!("{PREFIX}{control_value}"));
        key(&mut cursor, &mut text, KeyCode::Char('文'));
        assert_eq!(text, format!("{PREFIX}{control_value}文"));
        key(&mut cursor, &mut text, KeyCode::Home);
        for _ in PREFIX.chars() {
            key(&mut cursor, &mut text, KeyCode::Right);
        }
        assert_eq!(cursor.position(&text), PREFIX.len());
        key(&mut cursor, &mut text, KeyCode::Delete);
        assert_eq!(text, format!("{PREFIX}文"));
    }
}

#[test]
fn control_prefills_are_visible_and_never_shift_an_index_inside_utf8() {
    for original in [
        "/tmp/\n中",
        "/tmp/\r\n中",
        "\x1b[2J中\t文",
        "\u{85}e\u{301}👩‍💻",
    ] {
        let mut text = original.to_owned();
        let mut cursor = Cursor::default();
        for _ in 0..=text.chars().count() {
            assert!(text.is_char_boundary(cursor.position(&text)));
            for width in SMALL_WIDTHS {
                let view = cursor.viewport(&text, width);
                assert!(view.text.chars().all(|value| !value.is_control()));
                assert!(Span::raw(view.text.as_ref()).width() <= usize::from(width));
                assert!(view.cursor_column.is_none_or(|column| column < width));
            }
            key(&mut cursor, &mut text, KeyCode::Left);
        }
        assert_eq!(text, original);
        cursor.reset();
        let view = cursor.viewport(&text, 80);
        if original.contains('\n') {
            assert!(view.text.contains("\\n"));
        }
        if original.contains('\t') {
            assert!(view.text.contains("\\t"));
        }
    }
}
