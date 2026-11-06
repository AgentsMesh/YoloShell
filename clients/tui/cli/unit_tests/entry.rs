use super::constants::{HELP_ARGUMENT, HUB, HUB_ARGUMENT, PROGRAM, VERSION_ARGUMENT};
use clap::error::ErrorKind;
use std::ffi::OsString;

#[test]
fn help_and_version_return_clap_outcomes_to_the_dispatcher() {
    for (argument, expected) in [
        (HELP_ARGUMENT, ErrorKind::DisplayHelp),
        (VERSION_ARGUMENT, ErrorKind::DisplayVersion),
    ] {
        let error = crate::run_from([PROGRAM, argument].map(OsString::from))
            .unwrap_err()
            .downcast::<clap::Error>()
            .unwrap();
        assert_eq!(error.kind(), expected);
        assert!(!error.use_stderr());
    }
}

#[test]
fn incomplete_connection_returns_usage_error_before_starting_a_runtime() {
    let error = crate::run_from([PROGRAM, HUB_ARGUMENT, HUB].map(OsString::from)).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(yolo_command_cli::CONNECTION_REQUIRES_HUB),
        "{error}"
    );
}
