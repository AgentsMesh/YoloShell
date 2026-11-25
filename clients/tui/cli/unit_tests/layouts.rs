use super::constants::*;
use crate::layouts::LayoutCommand;
use crate::{arguments::Arguments, command::Command, selection::Selection};
use clap::Parser;

#[test]
fn explicit_layout_is_carried_through_local_and_explicit_connections() {
    let parsed = Arguments::try_parse_from([PROGRAM, LAYOUT_ARGUMENT, LAYOUT_ID]).unwrap();
    assert!(
        matches!(parsed.selection().unwrap(), Selection::Local { layout: Some(id), .. } if id.to_string() == LAYOUT_ID)
    );
    let parsed = Arguments::try_parse_from([
        PROGRAM,
        HUB_ARGUMENT,
        HUB,
        SOCKET_ARGUMENT,
        SOCKET,
        LAYOUT_ARGUMENT,
        LAYOUT_ID,
    ])
    .unwrap();
    let Selection::Explicit(options) = parsed.selection().unwrap() else {
        panic!()
    };
    assert_eq!(options.layout.unwrap().to_string(), LAYOUT_ID);
    assert!(Arguments::try_parse_from([PROGRAM, LAYOUT_ARGUMENT, "invalid"]).is_err());
    assert!(
        Arguments::try_parse_from([PROGRAM, LAYOUT_ARGUMENT, NIL_UUID])
            .unwrap()
            .selection()
            .is_err()
    );
}

#[test]
fn shared_layout_discovery_is_a_noninteractive_command() {
    let parsed = Arguments::try_parse_from([
        PROGRAM,
        "layout",
        LIST_COMMAND,
        JSON_ARGUMENT,
    ])
    .unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::Layout {
            command: LayoutCommand::List { json: true }
        })
    ));
}
