use super::reset_activation_retry_window;

#[test]
fn only_the_transition_into_pending_restarts_the_retry_window() {
    assert!(reset_activation_retry_window(false, true, true));
    assert!(!reset_activation_retry_window(true, true, true));
    assert!(!reset_activation_retry_window(false, true, false));
    assert!(!reset_activation_retry_window(false, false, true));
}
