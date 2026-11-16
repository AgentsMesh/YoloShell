use crate::fixtures::*;

#[test]
fn wide_and_combining_characters_are_copied_whole_from_either_cell() {
    let parser = parser(UNICODE_TEXT.as_bytes());
    for column in [NEXT, THIRD] {
        let selection = selected(&parser, point(FIRST, column), point(FIRST, column));
        assert_eq!(selection.text().unwrap(), "界");
        assert_eq!(selection.range(), (point(FIRST, NEXT), point(FIRST, THIRD)));
    }
    let selection = selected(&parser, point(FIRST, FOURTH), point(FIRST, NEXT));
    assert_eq!(selection.text().unwrap(), "界e\u{301}");
}

#[test]
fn soft_wraps_join_but_hard_newlines_remain() {
    let soft = parser(SOFT_TEXT);
    let selection = selected(&soft, point(FIRST, FIRST), point(THIRD, FIRST));
    assert_eq!(selection.text().unwrap(), "abcdefghijklmnopq");
    let hard = parser(HARD_TEXT);
    let selection = selected(&hard, point(FIRST, FIRST), point(NEXT, NEXT));
    assert_eq!(selection.text().unwrap(), "ab\ncd");
}

#[test]
fn blank_padding_is_trimmed_but_internal_blanks_and_explicit_spaces_remain() {
    let parser = parser(b"a\x1b[4Gz \r\n\r\nb");
    let selection = selected(&parser, point(FIRST, FIRST), point(THIRD, FIRST));
    assert_eq!(selection.text().unwrap(), "a  z \n\nb");
    let empty = selected(&parser, point(FOURTH, FIRST), point(FOURTH, LAST_COLUMN));
    assert_eq!(empty.text(), Err(SelectionError::EmptySelection));
}

#[test]
fn cropped_columns_do_not_join_across_hidden_text_or_copy_a_clipped_wide_character() {
    let soft = parser(SOFT_TEXT);
    let mut cropped = Selection::begin(soft.screen(), ROWS, FOURTH, point(FIRST, FIRST)).unwrap();
    cropped.finish(point(NEXT, THIRD)).unwrap();
    assert_eq!(cropped.text().unwrap(), "abc\nijk");
    let wide = parser(UNICODE_TEXT.as_bytes());
    let mut clipped = Selection::begin(wide.screen(), ROWS, THIRD, point(FIRST, NEXT)).unwrap();
    clipped.finish(point(FIRST, NEXT)).unwrap();
    assert_eq!(clipped.text(), Err(SelectionError::EmptySelection));
}
