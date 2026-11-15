use super::constants::*;
use crate::arguments::Arguments;
use crate::lifecycle::{select, settled};
use clap::Parser;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::{
    RuntimeStatus, RuntimeSummary, TerminalPresentation, TerminalSummary, TerminalSummaryFields,
};

fn base_fields(hub: u128, id: u128, name: &str, archived: bool) -> TerminalSummaryFields {
    TerminalSummaryFields {
        address: TerminalAddress {
            hub_id: HubId::from_uuid(uuid::Uuid::from_u128(hub)).unwrap(),
            terminal_id: TerminalId::from_uuid(uuid::Uuid::from_u128(id)).unwrap(),
        },
        name: name.to_owned(),
        presentation: TerminalPresentation::default(),
        cwd: "/tmp".to_owned(),
        archived,
        runtime: None,
        agent: None,
        recovery: None,
        workspace: None,
    }
}

fn terminal(hub: u128, id: u128, name: &str, archived: bool) -> TerminalSummary {
    TerminalSummary::new(base_fields(hub, id, name, archived)).unwrap()
}

fn with_status(status: RuntimeStatus) -> TerminalSummary {
    let mut fields = base_fields(7, 9, NAME, false);
    fields.runtime = Some(RuntimeSummary {
        target: RuntimeTarget {
            address: fields.address,
            runtime_epoch: RuntimeEpoch::from_uuid(uuid::Uuid::from_u128(5)).unwrap(),
        },
        status,
        output_sequence: 0,
        size: None,
        safe_termination: None,
    });
    TerminalSummary::new(fields).unwrap()
}

#[test]
fn archive_and_terminate_parse_with_their_flags() {
    let arguments = Arguments::try_parse_from([PROGRAM, ARCHIVE_COMMAND, NAME]).unwrap();
    assert!(matches!(
        arguments.command,
        Some(crate::command::Command::Archive {
            terminal,
            json: false
        }) if terminal == NAME
    ));
    let arguments =
        Arguments::try_parse_from([PROGRAM, TERMINATE_COMMAND, NAME, FORCE_ARGUMENT]).unwrap();
    assert!(matches!(
        arguments.command,
        Some(crate::command::Command::Terminate {
            terminal,
            force: true,
            json: false
        }) if terminal == NAME
    ));
    assert!(Arguments::try_parse_from([PROGRAM, ARCHIVE_COMMAND]).is_err());
}

#[test]
fn select_matches_label_id_address_and_short_form() {
    let terminals = vec![terminal(7, 9, NAME, false)];
    let fields = terminals[0].fields();
    for selector in [
        fields.name.clone(),
        fields.address.terminal_id.to_string(),
        fields.address.to_string(),
        format!("{}/{}", fields.address.hub_id, fields.address.terminal_id),
    ] {
        assert_eq!(
            select(&terminals, &selector).unwrap().fields().address,
            fields.address
        );
    }
}

#[test]
fn terminate_waits_for_an_exit_row_not_an_unreachable_reply() {
    for (status, expected) in [
        (RuntimeStatus::Running, false),
        (RuntimeStatus::Unreachable, false),
        (RuntimeStatus::Exited, true),
        (RuntimeStatus::Lost, true),
    ] {
        assert_eq!(settled(&with_status(status)), expected);
    }
}

#[test]
fn select_rejects_missing_ambiguous_and_archived_terminals() {
    let terminals = vec![
        terminal(1, 1, "same", false),
        terminal(1, 2, "same", false),
        terminal(1, 3, "history", true),
    ];
    assert!(select(&terminals, "same").is_err());
    assert!(select(&terminals, "missing").is_err());
    assert!(select(&terminals, "history").is_err());
}
