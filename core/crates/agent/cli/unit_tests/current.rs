use crate::current::selector_from;

#[test]
fn a_current_selector_requires_a_non_empty_terminal_id() {
    assert!(selector_from(None).is_err());
    assert!(selector_from(Some(String::new())).is_err());
    assert!(selector_from(Some("   ".into())).is_err());
    assert_eq!(selector_from(Some("terminal".into())).unwrap(), "terminal");
}
