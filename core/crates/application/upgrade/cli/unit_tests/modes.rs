use crate::Cli;
use crate::constants::*;
use clap::Parser;

fn parse(extra: &[&str]) -> crate::Options {
    let mut args = vec![PROGRAM, COMMAND];
    args.extend_from_slice(extra);
    let parsed = Cli::try_parse_from(args).unwrap();
    let crate::Command::Update(options) = parsed.command;
    options
}

#[test]
fn update_modes_are_explicit_and_install_activation_is_the_default() {
    let update = parse(&[]);
    assert!(!update.check && !update.admit && !update.restart && !update.install_release);
    assert!(parse(&[CHECK_SWITCH]).check);
    assert!(parse(&[CHECK_RELEASE_SWITCH]).check);
    assert!(parse(&[ADMIT_SWITCH]).admit);
    assert!(parse(&[INSTALL_RELEASE_SWITCH]).install_release);
    assert!(parse(&[NO_RESTART_SWITCH]).no_restart);
    assert!(parse(&[RESTART_SWITCH]).restart);
}

#[test]
fn incompatible_update_modes_are_rejected_at_parse_time() {
    for modes in [
        [CHECK_SWITCH, ADMIT_SWITCH],
        [CHECK_SWITCH, NO_RESTART_SWITCH],
        [ADMIT_SWITCH, RESTART_SWITCH],
        [INSTALL_RELEASE_SWITCH, NO_RESTART_SWITCH],
        [INSTALL_RELEASE_SWITCH, RESTART_SWITCH],
    ] {
        let mut args = vec![PROGRAM, COMMAND];
        args.extend_from_slice(&modes);
        assert_eq!(
            Cli::try_parse_from(args).unwrap_err().kind(),
            clap::error::ErrorKind::ArgumentConflict,
            "{modes:?}"
        );
    }
}
