use super::{fixtures::*, protocol::arguments};
use crate::{Arguments, constants::*};
use clap::Parser;

#[test]
fn command_descriptor_requires_an_exact_launch_and_disjoint_status_descriptor() {
    for (command, launch, valid) in [
        (Some(COMMAND_FD), Some(HOLDER), true),
        (None, None, true),
        (Some(STATUS_FD), Some(HOLDER), false),
        (Some(STDERR_FD), Some(HOLDER), false),
        (Some(COMMAND_FD), Some(NIL_ID), false),
    ] {
        let mut args = Arguments::try_parse_from(arguments()).unwrap();
        args.status_fd = Some(STATUS_FD);
        args.command_fd = command;
        args.launch_id = launch.map(|text| text.parse().unwrap());
        assert_eq!(args.validate_channels().is_ok(), valid);
    }
}

#[test]
fn command_cli_options_must_be_supplied_together() {
    for (key, value) in [(COMMAND_OPTION, COMMAND_FD_TEXT), (LAUNCH_OPTION, HOLDER)] {
        let mut args = arguments();
        args.extend([format!("{OPTION_PREFIX}{key}"), value.into()]);
        assert!(Arguments::try_parse_from(args).is_err());
    }
    let mut args = arguments();
    for (key, value) in [(COMMAND_OPTION, COMMAND_FD_TEXT), (LAUNCH_OPTION, HOLDER)] {
        args.extend([format!("{OPTION_PREFIX}{key}"), value.into()]);
    }
    let parsed = Arguments::try_parse_from(args).unwrap();
    parsed.validate_channels().unwrap();
    assert_eq!(parsed.command_fd, Some(COMMAND_FD));
    assert_eq!(parsed.launch_id.unwrap().to_string(), HOLDER);
}
