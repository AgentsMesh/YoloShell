use super::*;
use ratatui::text::Span;

#[test]
fn long_line_scrolls_to_the_edit_position_and_back_to_its_beginning() {
    let mut cursor = Cursor::default();
    let mut text = "/a/long/path/to/a/目录".to_owned();
    let end = cursor.viewport(&text, 8);
    assert!(end.text.ends_with("目录"));
    assert!(end.cursor_column.unwrap() < 8);
    key(&mut cursor, &mut text, KeyCode::Home);
    assert!(cursor.viewport(&text, 8).text.starts_with("/a/long/"));
    assert_eq!(cursor.viewport(&text, 8).cursor_column, Some(0));
}

#[test]
fn every_small_width_keeps_unicode_and_cursor_inside_the_view() {
    let mut cursor = Cursor::default();
    let mut text = "中e\u{301}👩‍💻Z".to_owned();
    for _ in 0..=text.chars().count() {
        for width in 0..=12 {
            let view = cursor.viewport(&text, width);
            assert!(Span::raw(view.text.as_ref()).width() <= usize::from(width));
            assert!(view.cursor_column.is_none_or(|column| column < width));
            assert_eq!(view.cursor_column.is_none(), width == 0);
            assert!(!view.text.starts_with('\u{301}'));
            assert!(!view.text.starts_with('\u{200d}'));
        }
        key(&mut cursor, &mut text, KeyCode::Left);
    }
}
