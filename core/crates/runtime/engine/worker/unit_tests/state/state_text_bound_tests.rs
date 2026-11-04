use yolo_terminal_model::MAX_TERMINAL_TEXT_BYTES;

const EURO_BYTES: usize = 3;
const EXTRA_CHAR: usize = 1;

#[test]
fn terminal_text_bound_truncates_on_a_utf8_boundary() {
    let exact = "a".repeat(MAX_TERMINAL_TEXT_BYTES);
    assert_eq!(super::bound_text(exact.clone()), (exact, false));

    let euro = "€".repeat(MAX_TERMINAL_TEXT_BYTES / EURO_BYTES + EXTRA_CHAR);
    let (text, truncated) = super::bound_text(euro);
    assert!(truncated);
    assert!(text.len() <= MAX_TERMINAL_TEXT_BYTES);
    assert!(text.chars().all(|character| character == '€'));
}
