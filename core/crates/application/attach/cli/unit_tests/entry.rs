#[path = "../test_support/entry_constants.rs"]
mod constants;
use clap::error::ErrorKind;
use constants::*;
use std::ffi::OsString;

#[test]
fn explicit_attach_argv_returns_before_executor_and_terminal_initialization() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.block_on(async {
        for (argument, expected) in [
            (HELP_ARGUMENT, ErrorKind::DisplayHelp),
            (VERSION_ARGUMENT, ErrorKind::DisplayVersion),
            (INVALID_ARGUMENT, ErrorKind::UnknownArgument),
        ] {
            let error = crate::run_from([PROGRAM_NAME, argument].map(OsString::from)).unwrap_err();
            let error = error.downcast::<clap::Error>().unwrap();
            assert_eq!(error.kind(), expected);
            if matches!(expected, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                assert!(error.to_string().contains(PROGRAM_NAME));
            }
        }
    });
}
