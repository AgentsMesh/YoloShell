use super::fixtures::{constants::*, *};
use crate::ConfigurationSnapshot;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn address(port: u16) -> SocketAddr {
    SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port)
}

#[test]
fn listener_snapshot_accepts_only_applied_actual_endpoint_or_unapplied_absence() {
    let valid = [
        (false, None, None),
        (false, Some(address(PORT)), None),
        (true, None, None),
        (true, Some(address(PORT)), Some(address(PORT))),
        (true, Some(address(ANY_PORT)), Some(address(PORT))),
    ];
    for (applied, listen, listen_address) in valid {
        let mut state = state();
        state.configuration.listen = listen;
        let snapshot = ConfigurationSnapshot {
            state,
            applied,
            listen_address,
        };
        snapshot.validate().unwrap();
        roundtrip(&snapshot);
        reject(&snapshot, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    }
}

#[test]
fn inconsistent_bindings_and_nested_invalid_state_are_rejected_on_wire() {
    let invalid = [
        (false, None, Some(address(PORT))),
        (false, Some(address(PORT)), Some(address(PORT))),
        (true, None, Some(address(PORT))),
        (true, Some(address(PORT)), None),
        (true, Some(address(PORT)), Some(address(OTHER_PORT))),
        (true, Some(address(ANY_PORT)), Some(address(ANY_PORT))),
        (
            true,
            Some(address(PORT)),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT)),
        ),
    ];
    for (applied, listen, listen_address) in invalid {
        let mut state = state();
        state.configuration.listen = listen;
        let snapshot = ConfigurationSnapshot {
            state,
            applied,
            listen_address,
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
            listen_address: None
        }
        .validate()
        .is_err()
    );
}
