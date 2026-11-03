use super::*;
use crate::packaged_claude_test_support::{SESSION_ID, strings};
use yolo_runtime::agent::claude::constants::{
    CONTINUE_ARGUMENT, RESUME_ARGUMENT, SESSION_ID_ARGUMENT,
};
const EXTRA: &str = "extra";
const INVALID: &str = "not-a-session";

#[test]
fn fresh_launch_generates_its_own_session_without_injected_arguments() {
    let first = parse(&[]).unwrap();
    let second = parse(&[]).unwrap();
    assert_eq!(first.mode, Mode::Fresh);
    assert_ne!(first.session_id, second.session_id);
    assert!(!uuid::Uuid::parse_str(&first.session_id).unwrap().is_nil());
    assert_eq!(first.mode.label(), PACKAGED_FRESH_MODE);
    assert_eq!(first.mode.source(), PACKAGED_START_SOURCE);
}

#[test]
fn exact_resume_preserves_provider_session_id() {
    let resumed = parse(&strings(&[RESUME_ARGUMENT, SESSION_ID])).unwrap();
    assert_eq!(resumed.mode, Mode::Resume);
    assert_eq!(resumed.session_id, SESSION_ID);
    assert_eq!(resumed.mode.label(), PACKAGED_RESUME_MODE);
    assert_eq!(resumed.mode.source(), PACKAGED_RESUME_SOURCE);
}

#[test]
fn ambiguous_and_injected_launch_arguments_are_rejected() {
    for arguments in [
        strings(&[SESSION_ID_ARGUMENT, SESSION_ID]),
        strings(&[RESUME_ARGUMENT]),
        strings(&[RESUME_ARGUMENT, INVALID]),
        strings(&[CONTINUE_ARGUMENT, SESSION_ID]),
        strings(&[RESUME_ARGUMENT, SESSION_ID, EXTRA]),
        strings(&[RESUME_ARGUMENT, &uuid::Uuid::nil().to_string()]),
    ] {
        assert!(parse(&arguments).is_err(), "{arguments:?}");
    }
}
