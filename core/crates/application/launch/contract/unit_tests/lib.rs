use std::{ffi::OsString, os::unix::ffi::OsStringExt};
use yolo_launch_contract::{EXECUTABLE_NAME, INTERNAL_COMMAND, Invocation, ProcessRole};

mod constants;
use constants::*;

#[test]
fn every_explicit_role_preserves_opaque_arguments_and_receives_its_own_usage_name() {
    for role in ProcessRole::ALL {
        let opaque = OsString::from_vec(INVALID_UTF8.to_vec());
        let invocation = Invocation::parse([
            ARGV0.into(),
            INTERNAL_COMMAND.into(),
            role.name().into(),
            ARGUMENT.into(),
            SPACED_PATH.into(),
            opaque.clone(),
        ])
        .unwrap();
        assert_eq!(invocation.role, Some(*role));
        assert_eq!(
            invocation.arguments,
            [
                format!("{EXECUTABLE_NAME} {INTERNAL_COMMAND} {}", role.name()).into(),
                ARGUMENT.into(),
                SPACED_PATH.into(),
                opaque,
            ]
        );
        assert_eq!(role.arguments(), [INTERNAL_COMMAND, role.name()]);
    }
}

#[test]
fn absent_or_invalid_internal_roles_are_rejected_without_falling_back_to_tui() {
    assert!(Invocation::parse(Vec::<OsString>::new()).is_err());
    assert!(Invocation::parse([ARGV0.into(), INTERNAL_COMMAND.into()]).is_err());
    for invalid in [
        UNKNOWN_ROLE.into(),
        REMOVED_DESKTOP_ROLE.into(),
        OsString::from_vec(INVALID_UTF8.to_vec()),
    ] {
        assert!(Invocation::parse([ARGV0.into(), INTERNAL_COMMAND.into(), invalid]).is_err());
    }
}

#[test]
fn default_tui_selection_ignores_program_basename_and_never_consumes_user_arguments() {
    for program in [ARGV0, UNKNOWN_ROLE, INTERNAL_COMMAND] {
        let invocation = Invocation::parse([program.into()]).unwrap();
        assert_eq!(invocation.role, None);
        assert_eq!(invocation.arguments, [OsString::from(EXECUTABLE_NAME)]);
        let invocation = Invocation::parse([
            program.into(),
            ARGUMENT.into(),
            INTERNAL_COMMAND.into(),
            SPACED_PATH.into(),
        ])
        .unwrap();
        assert_eq!(invocation.role, None);
        assert_eq!(
            invocation.arguments,
            [
                OsString::from(EXECUTABLE_NAME),
                ARGUMENT.into(),
                INTERNAL_COMMAND.into(),
                SPACED_PATH.into(),
            ]
        );
    }
}
