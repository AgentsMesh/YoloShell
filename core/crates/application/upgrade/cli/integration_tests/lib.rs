use std::ffi::OsString;

mod constants;
use constants::{PROGRAM, UPDATE_COMMAND};

#[test]
fn update_command_is_selected_before_the_interactive_client() {
    let arguments: Vec<OsString> = [PROGRAM, UPDATE_COMMAND]
        .into_iter()
        .map(OsString::from)
        .collect();
    assert!(yolo_upgrade_cli::selects(&arguments));
}
