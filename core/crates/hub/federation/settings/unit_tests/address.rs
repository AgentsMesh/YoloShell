use super::address_constants::*;
use std::path::{Path, PathBuf};
use yolo_hub_federation_settings::{MAX_SOCKET_PATH_BYTES, PeerAddress};

fn socket(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
}

#[test]
fn ssh_peer_address_roundtrips_with_an_absolute_socket() {
    let address = PeerAddress::ssh(PROFILE, socket(SOCKET)).unwrap();
    assert_eq!(address.socket, socket(SOCKET));
    let decoded: PeerAddress =
        serde_json::from_value(serde_json::to_value(&address).unwrap()).unwrap();
    assert_eq!(decoded, address);
    for path in INVALID_SOCKETS {
        assert!(
            PeerAddress::ssh(PROFILE, socket(path)).is_err(),
            "{path:?}"
        );
    }
}

#[test]
fn the_socket_budget_stays_inside_the_kernel_sun_path_limit() {
    let acceptable = format!("/{}", "a".repeat(MAX_SOCKET_PATH_BYTES - 1));
    assert!(PeerAddress::ssh(PROFILE, socket(&acceptable)).is_ok());
    let overlong = format!("/{}", "a".repeat(MAX_SOCKET_PATH_BYTES));
    assert!(PeerAddress::ssh(PROFILE, socket(&overlong)).is_err());
}

#[test]
fn a_peer_address_cannot_drop_its_ssh_profile_binding() {
    let address = PeerAddress::ssh(PROFILE, socket(SOCKET)).unwrap();
    let mut value = serde_json::to_value(&address).unwrap();
    value.as_object_mut().unwrap().remove(PROFILE_FIELD);
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
    let mut value = serde_json::to_value(&address).unwrap();
    value["socket"] = "peer.sock".into();
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
}
