use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn confirmed_exec_permanently_disarms_explicit_failure() {
    let state = BootstrapExecState::armed();
    let authority = state.take_authority().unwrap();
    authority.confirm_exec().unwrap();

    assert_eq!(state.shared().unwrap().load(), Transition::ExecConfirmed);
    assert!(state.take_authority().is_err());
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn explicit_failure_is_typed_and_consumed_once() {
    let state = BootstrapExecState::armed();
    let failure = state
        .take_authority()
        .unwrap()
        .confirm_explicit_failure()
        .unwrap();

    state.consume_failure(failure).unwrap();
    assert_eq!(state.shared().unwrap().load(), Transition::FailureConsumed);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn pre_release_cancellation_is_typed_and_consumed_once() {
    let state = BootstrapExecState::armed();
    let cancellation = state
        .take_authority()
        .unwrap()
        .cancel_before_release()
        .unwrap();

    state.consume_failure(cancellation).unwrap();
    assert_eq!(state.shared().unwrap().load(), Transition::FailureConsumed);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn dropped_unsettled_authority_becomes_indeterminate() {
    let state = BootstrapExecState::armed();
    drop(state.take_authority().unwrap());

    assert_eq!(state.shared().unwrap().load(), Transition::Indeterminate);
}
