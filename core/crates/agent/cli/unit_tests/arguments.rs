use super::constants::*;
use crate::arguments::{AgentCommand, Arguments, Command};
use crate::constants::{COMMAND, PROGRAM, STATE_IDLE};
use clap::Parser;
use std::ffi::OsString;
use uuid::Uuid;
use yolo_hub_identity::HubId;

pub(super) fn base() -> Vec<OsString> {
    vec![
        PROGRAM.into(),
        COMMAND.into(),
        SOCKET_FLAG.into(),
        "/tmp/hub.sock".into(),
        HUB_FLAG.into(),
        HubId::new().to_string().into(),
    ]
}

#[test]
fn selects_only_the_agent_command() {
    for command in [COMMAND, UNMATCHED, CONVERSATION_COMMAND] {
        let argv = [OsString::from(PROGRAM), OsString::from(command)];
        assert_eq!(crate::selects(&argv), command == COMMAND);
    }
    assert!(!crate::selects(&[OsString::from(PROGRAM)]));
}

#[test]
fn list_parses_with_global_connection_flags() {
    let mut argv = base();
    argv.extend([LIST_ACTION.into(), JSON_FLAG.into()]);
    let Command::Agent(options) = Arguments::try_parse_from(argv).unwrap().command;
    assert!(options.socket.is_some());
    assert!(options.hub.is_some());
    let AgentCommand::List(list) = options.command else {
        panic!("expected list")
    };
    assert!(list.output.json);
}

#[test]
fn current_can_replace_a_selector_but_conflicts_with_one() {
    let mut get = base();
    get.extend([GET_ACTION.into(), CURRENT_FLAG.into()]);
    let Command::Agent(options) = Arguments::try_parse_from(get).unwrap().command;
    let AgentCommand::Get {
        selector,
        current,
        output,
    } = options.command
    else {
        panic!("expected get")
    };
    assert!(selector.is_none());
    assert!(current);
    assert!(!output.json);

    let mut conflict = base();
    conflict.extend([GET_ACTION.into(), CURRENT_FLAG.into(), "flow-1".into()]);
    assert!(Arguments::try_parse_from(conflict).is_err());

    let mut send = base();
    send.extend([SEND_ACTION.into(), CURRENT_FLAG.into(), STDIN_FLAG.into()]);
    let Command::Agent(options) = Arguments::try_parse_from(send).unwrap().command;
    let AgentCommand::Send(args) = options.command else {
        panic!("expected send")
    };
    assert!(args.selector.is_none());
    assert!(args.current);

    let mut wait = base();
    wait.extend([
        WAIT_ACTION.into(),
        CURRENT_FLAG.into(),
        UNTIL_FLAG.into(),
        IDLE_STATE.into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(wait).unwrap().command;
    let AgentCommand::Wait(args) = options.command else {
        panic!("expected wait")
    };
    assert!(args.selectors.is_empty());
    assert!(args.current);
}

#[test]
fn a_prompt_or_stdin_is_required() {
    let mut missing = base();
    missing.extend([ASK_ACTION.into(), "flow-1".into()]);
    assert!(Arguments::try_parse_from(missing).is_err());

    let mut conflicting = base();
    conflicting.extend([
        ASK_ACTION.into(),
        "flow-1".into(),
        "hello".into(),
        STDIN_FLAG.into(),
    ]);
    assert!(Arguments::try_parse_from(conflicting).is_err());

    let mut zero = base();
    zero.extend([
        ASK_ACTION.into(),
        "flow-1".into(),
        "hello".into(),
        TIMEOUT_FLAG.into(),
        "0".into(),
    ]);
    assert!(Arguments::try_parse_from(zero).is_err());

    let mut accepted = base();
    accepted.extend([
        ASK_ACTION.into(),
        "flow-1".into(),
        "hello".into(),
        TIMEOUT_FLAG.into(),
        "30".into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(accepted).unwrap().command;
    let AgentCommand::Ask(ask) = options.command else {
        panic!("expected ask")
    };
    assert_eq!(ask.timeout, 30);
}

#[test]
fn an_explicit_socket_requires_a_hub_identity() {
    let hub = HubId::new().to_string();
    assert!(
        Arguments::try_parse_from([PROGRAM, COMMAND, SOCKET_FLAG, "/tmp/hub.sock", LIST_ACTION,])
            .is_err()
    );
    assert!(Arguments::try_parse_from([PROGRAM, COMMAND, HUB_FLAG, &hub, LIST_ACTION]).is_err());
}

#[test]
fn start_current_inherits_a_workspace_and_conflicts_with_one() {
    let mut inherit = base();
    inherit.extend([
        START_ACTION.into(),
        PROVIDER_FLAG.into(),
        "codex".into(),
        CURRENT_FLAG.into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(inherit).unwrap().command;
    let AgentCommand::Start(start) = options.command else {
        panic!("expected start")
    };
    assert!(start.current);
    assert!(start.workspace.is_none());

    let mut conflict = base();
    conflict.extend([
        START_ACTION.into(),
        PROVIDER_FLAG.into(),
        "codex".into(),
        CURRENT_FLAG.into(),
        WORKSPACE_FLAG.into(),
        Uuid::from_u128(7).to_string().into(),
    ]);
    assert!(Arguments::try_parse_from(conflict).is_err());
}

#[test]
fn stop_parses_a_run_name_and_force_flag() {
    let mut argv = base();
    argv.extend([
        "stop".into(),
        "alpha-run".into(),
        FORCE_FLAG.into(),
        JSON_FLAG.into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(argv).unwrap().command;
    let AgentCommand::Stop(stop) = options.command else {
        panic!("expected stop")
    };
    assert_eq!(stop.run, "alpha-run");
    assert!(stop.force);
    assert!(stop.output.json);
}

#[test]
fn wait_accepts_run_selectors_without_positional_selectors() {
    let mut argv = base();
    argv.extend([
        WAIT_ACTION.into(),
        RUN_FLAG.into(),
        "alpha".into(),
        UNTIL_FLAG.into(),
        STATE_IDLE.into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(argv).unwrap().command;
    let AgentCommand::Wait(wait) = options.command else {
        panic!("expected wait")
    };
    assert!(wait.selectors.is_empty());
    assert_eq!(wait.runs, vec!["alpha".to_owned()]);
}
