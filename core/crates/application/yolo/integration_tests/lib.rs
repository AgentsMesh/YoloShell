#[path = "../test_support/command.rs"]
mod command;
#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use yolo_launch_contract::{EXECUTABLE_NAME, INTERNAL_COMMAND, ProcessRole};
mod agent_commands;
mod conversations;
mod discovery;

#[tokio::test]
async fn one_image_provides_tui_and_every_explicit_service_entry() {
    let version = command::run(&[VERSION], None).await;
    assert_eq!(version.code, SUCCESS);
    assert!(version.stdout.starts_with(&format!("{EXECUTABLE_NAME} ")));
    assert!(version.stderr.is_empty());
    for role in ProcessRole::ALL {
        let version = command::run(&[INTERNAL_COMMAND, role.name(), VERSION], None).await;
        assert_eq!(version.code, SUCCESS, "{}", version.stderr);
        assert!(
            version
                .stdout
                .starts_with(&format!("{} ", role.display_name()))
        );
        assert!(version.stderr.is_empty());
        let help = command::run(&[INTERNAL_COMMAND, role.name(), HELP], None).await;
        assert_eq!(help.code, SUCCESS, "{}", help.stderr);
        assert!(!help.stdout.is_empty());
        assert!(help.stderr.is_empty());
    }
}

#[tokio::test]
async fn malformed_dispatch_and_role_options_fail_without_entering_tui() {
    for arguments in [
        vec![INTERNAL_COMMAND],
        vec![INTERNAL_COMMAND, UNKNOWN_ROLE],
        vec![INVALID_OPTION],
        vec![INTERNAL_COMMAND, ProcessRole::Host.name(), INVALID_OPTION],
        vec![
            INTERNAL_COMMAND,
            ProcessRole::FileHelper.name(),
            INVALID_OPTION,
        ],
    ] {
        let result = command::run(&arguments, None).await;
        assert_eq!(result.code, USAGE);
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
}

#[tokio::test]
async fn executable_basename_never_selects_a_service_role() {
    let result = command::run(&[VERSION], Some(RETIRED_ENTRY)).await;
    assert_eq!(result.code, SUCCESS);
    assert!(result.stdout.starts_with(&format!("{EXECUTABLE_NAME} ")));
    assert!(result.stderr.is_empty());
}

#[tokio::test]
async fn file_helper_protocol_failures_retain_the_silent_error_boundary() {
    let result = command::run(&ProcessRole::FileHelper.arguments(), None).await;
    assert_eq!(result.code, FAILURE);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}
