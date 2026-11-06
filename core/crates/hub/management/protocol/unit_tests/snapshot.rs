use super::fixtures::{constants::*, *};
use crate::ConfigurationSnapshot;
use std::path::PathBuf;

const SOCKET: &str = "/run/user/1000/yolo/hub/peer.sock";

#[test]
fn snapshot_accepts_only_an_applied_peer_socket_or_unapplied_absence() {
    for (applied, peer_socket) in [(false, None), (true, Some(PathBuf::from(SOCKET)))] {
        let snapshot = ConfigurationSnapshot {
            state: state(),
            applied,
            peer_socket,
        };
        snapshot.validate().unwrap();
        roundtrip(&snapshot);
        reject(&snapshot, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    }
}

#[test]
fn inconsistent_peer_sockets_and_nested_invalid_state_are_rejected_on_wire() {
    for (applied, peer_socket) in [
        (false, Some(PathBuf::from(SOCKET))),
        (true, None),
        (true, Some(PathBuf::from("peer.sock"))),
        (true, Some(PathBuf::from("/tmp/../peer.sock"))),
        (true, Some(PathBuf::from("/tmp/peer\n.sock"))),
        (true, Some(PathBuf::from(format!("/{}", "a".repeat(100))))),
    ] {
        let snapshot = ConfigurationSnapshot {
            state: state(),
            applied,
            peer_socket,
        };
        assert!(snapshot.validate().is_err());
        assert!(
            serde_json::from_value::<ConfigurationSnapshot>(
                serde_json::to_value(snapshot).unwrap()
            )
            .is_err()
        );
    }
    let mut state = state();
    state.revision = 0;
    assert!(
        ConfigurationSnapshot {
            state,
            applied: false,
            peer_socket: None
        }
        .validate()
        .is_err()
    );
}
