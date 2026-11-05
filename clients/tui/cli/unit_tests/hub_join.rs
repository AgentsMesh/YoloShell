use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use yolo_tui_hub_cli::{HubCommand, PeerCommand};

#[test]
fn join_and_identity_are_local_hub_commands() {
    let parsed =
        Arguments::try_parse_from([PROGRAM, HUB_COMMAND, PEER_COMMAND, JOIN_COMMAND, SSH_TARGET])
            .unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::Hub {
            command: HubCommand::Peer {
                command: PeerCommand::Join(_)
            }
        })
    ));
    let parsed =
        Arguments::try_parse_from([PROGRAM, HUB_COMMAND, IDENTITY_COMMAND, JSON_ARGUMENT]).unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::Hub {
            command: HubCommand::Identity { json: true }
        })
    ));
}
