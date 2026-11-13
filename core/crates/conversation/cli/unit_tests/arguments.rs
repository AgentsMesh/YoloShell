use super::constants::*;
use crate::constants::{COMMAND, PROGRAM};
use crate::{arguments::*, selects};
use clap::Parser;
use std::ffi::OsString;
use yolo_hub_identity::{AgentId, HubId};

fn base() -> Vec<String> {
    vec![
        PROGRAM.into(),
        COMMAND.into(),
        SOCKET_FLAG.into(),
        "/tmp/explicit-entry.sock".into(),
        HUB_FLAG.into(),
        HubId::new().to_string(),
    ]
}

#[test]
fn dispatch_selects_only_the_conversation_command() {
    for command in [COMMAND, ATTACH_ACTION, PLUGINS_ACTION, "conversations"] {
        let argv = [OsString::from(PROGRAM), OsString::from(command)];
        assert_eq!(selects(&argv), command == COMMAND);
    }
    assert!(!selects(&[]));
}

#[test]
fn current_agent_resolve_and_read_need_no_terminal_or_execution_uuid() {
    for command in [RESOLVE_ACTION, READ_ACTION] {
        let mut argv = base();
        argv.extend([
            command.into(),
            AGENT_FLAG.into(),
            AgentId::new().to_string(),
            ORIGIN_FLAG.into(),
            HubId::new().to_string(),
        ]);
        Arguments::try_parse_from(argv).unwrap();
    }
    let mut argv = base();
    argv.push(READ_ACTION.into());
    let Command::Conversation(options) = Arguments::try_parse_from(argv).unwrap().command;
    assert!(matches!(
        options.action,
        Action::Read {
            agent: None,
            origin: None
        }
    ));
}

#[test]
fn watch_is_bounded_and_entry_identity_is_required() {
    let mut argv = base();
    argv.push(WATCH_ACTION.into());
    let Command::Conversation(options) = Arguments::try_parse_from(argv.clone()).unwrap().command;
    assert!(matches!(
        options.action,
        Action::Watch {
            wait_ms: yolo_conversation_protocol::MAX_WATCH_WAIT_MS
        }
    ));
    argv.extend([
        WAIT_FLAG.into(),
        (yolo_conversation_protocol::MAX_WATCH_WAIT_MS + 1).to_string(),
    ]);
    assert!(Arguments::try_parse_from(argv).is_err());
    let Command::Conversation(options) =
        Arguments::try_parse_from([PROGRAM, COMMAND, READ_ACTION])
            .unwrap()
            .command;
    assert!(options.socket.is_none() && options.hub.is_none() && options.local_profile.is_none());
    let Command::Conversation(options) = Arguments::try_parse_from([
        PROGRAM,
        COMMAND,
        LOCAL_PROFILE_FLAG,
        PROFILE_PATH,
        READ_ACTION,
    ])
    .unwrap()
    .command;
    assert!(matches!(
        options.local_profile,
        Some(path) if path.to_str() == Some(PROFILE_PATH)
    ));
    assert!(Arguments::try_parse_from([PROGRAM, COMMAND, SOCKET_FLAG, "/tmp/x.sock", READ_ACTION]).is_err());
    let hub = HubId::new().to_string();
    assert!(
        Arguments::try_parse_from([PROGRAM, COMMAND, HUB_FLAG, hub.as_str(), READ_ACTION])
            .is_err()
    );
    let mut argv = base();
    argv.extend([LOCAL_PROFILE_FLAG.into(), PROFILE_PATH.into()]);
    assert!(Arguments::try_parse_from(argv).is_err());
    let mut argv = base();
    argv.extend([
        READ_ACTION.into(),
        ORIGIN_FLAG.into(),
        HubId::new().to_string(),
    ]);
    assert!(Arguments::try_parse_from(argv).is_err());
}
