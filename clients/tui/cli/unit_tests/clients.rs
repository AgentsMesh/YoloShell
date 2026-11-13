use super::constants::*;
use crate::constants::{CLIENT_HEADER, CLIENT_KIND_CLIENT, CLIENT_KIND_PRIMARY};
use crate::detach::{client_rows, clients_output};
use crate::{arguments::Arguments, command::Command};
use clap::Parser;
use std::path::PathBuf;
use yolo_hub_identity::HubId;
use yolo_tui_tab::ClientDirectory;

#[test]
fn the_hub_bound_client_is_reported_as_primary() {
    let hub = HubId::new();
    let other = uuid::Uuid::new_v4();
    let active = vec![
        ClientDirectory {
            id: other,
            path: PathBuf::from("/tmp/client"),
        },
        ClientDirectory {
            id: hub.as_uuid(),
            path: PathBuf::from("/tmp/hub"),
        },
    ];
    let rows = client_rows(&active, hub);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, hub.as_uuid());
    assert_eq!(rows[0].kind, CLIENT_KIND_PRIMARY);
    assert_eq!(rows[1].kind, CLIENT_KIND_CLIENT);
    assert_eq!(
        clients_output(&rows, false),
        format!(
            "{CLIENT_HEADER}\n{}\t{}\n{}\t{}\n",
            hub.as_uuid(),
            CLIENT_KIND_PRIMARY,
            other,
            CLIENT_KIND_CLIENT
        )
    );
    let json: serde_json::Value = serde_json::from_str(&clients_output(&rows, true)).unwrap();
    assert_eq!(json[0]["kind"], CLIENT_KIND_PRIMARY);
    assert_eq!(json[0]["id"], hub.as_uuid().to_string());
    assert_eq!(json[1]["kind"], CLIENT_KIND_CLIENT);
}

#[test]
fn client_ids_remain_addressable_from_the_printed_table() {
    let hub = HubId::new();
    let other = uuid::Uuid::new_v4();
    let active = vec![ClientDirectory {
        id: other,
        path: PathBuf::from("/tmp/client"),
    }];
    let rows = client_rows(&active, hub);
    let text = clients_output(&rows, false);
    assert!(text.contains(&other.to_string()));
    assert_eq!(rows[0].kind, CLIENT_KIND_CLIENT);
}

#[test]
fn list_and_clients_expose_history_and_machine_readable_modes() {
    let parsed =
        Arguments::try_parse_from([PROGRAM, LIST_COMMAND, ALL_ARGUMENT, JSON_ARGUMENT]).unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::List {
            json: true,
            all: true
        })
    ));
    let parsed = Arguments::try_parse_from([PROGRAM, LIST_COMMAND]).unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::List {
            json: false,
            all: false
        })
    ));
    let parsed = Arguments::try_parse_from([PROGRAM, CLIENTS_COMMAND, JSON_ARGUMENT]).unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::Clients { json: true })
    ));
    let parsed = Arguments::try_parse_from([PROGRAM, CLIENTS_COMMAND]).unwrap();
    assert!(matches!(
        parsed.command,
        Some(Command::Clients { json: false })
    ));
}
