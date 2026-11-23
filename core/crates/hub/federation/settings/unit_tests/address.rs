use super::address_constants::*;
use std::path::{Path, PathBuf};
use yolo_hub_federation_settings::{MAX_SOCKET_PATH_BYTES, MAX_SSH_TARGET_BYTES, PeerAddress};

fn socket(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
}

#[test]
fn ssh_peer_address_roundtrips_with_an_absolute_socket() {
    let address = PeerAddress::ssh(SSH_TARGET, socket(SOCKET)).unwrap();
    assert_eq!(address.socket(), socket(SOCKET));
    assert_eq!(address.ssh_target(), Some(SSH_TARGET));
    let decoded: PeerAddress =
        serde_json::from_value(serde_json::to_value(&address).unwrap()).unwrap();
    assert_eq!(decoded, address);
    for path in INVALID_SOCKETS {
        assert!(
            PeerAddress::ssh(SSH_TARGET, socket(path)).is_err(),
            "{path:?}"
        );
    }
}

#[test]
fn ssh_targets_cannot_be_options_or_shell_fragments() {
    for target in ["", "-oProxyCommand=x", "user host", "user@host;rm", "a/b", "a\nb"] {
        assert!(
            PeerAddress::ssh(target, socket(SOCKET)).is_err(),
            "{target:?}"
        );
    }
    assert!(PeerAddress::ssh("a".repeat(MAX_SSH_TARGET_BYTES + 1), socket(SOCKET)).is_err());
    assert!(PeerAddress::ssh("user@host", socket(SOCKET)).is_ok());
    assert!(PeerAddress::ssh("host-alias_1.example", socket(SOCKET)).is_ok());
}

#[test]
fn profile_dials_remain_readable_for_managed_remote_setup() {
    let address = PeerAddress::profile(PROFILE_ID, socket(SOCKET)).unwrap();
    assert_eq!(address.profile_id(), Some(PROFILE_ID));
    let decoded: PeerAddress =
        serde_json::from_value(serde_json::to_value(&address).unwrap()).unwrap();
    assert_eq!(decoded, address);
}

#[test]
fn the_socket_budget_stays_inside_the_kernel_sun_path_limit() {
    let acceptable = format!("/{}", "a".repeat(MAX_SOCKET_PATH_BYTES - 1));
    assert!(PeerAddress::ssh(SSH_TARGET, socket(&acceptable)).is_ok());
    let overlong = format!("/{}", "a".repeat(MAX_SOCKET_PATH_BYTES));
    assert!(PeerAddress::ssh(SSH_TARGET, socket(&overlong)).is_err());
}

#[test]
fn a_peer_address_cannot_drop_its_transport_binding() {
    let address = PeerAddress::ssh(SSH_TARGET, socket(SOCKET)).unwrap();
    let mut value = serde_json::to_value(&address).unwrap();
    value.as_object_mut().unwrap().remove(SSH_FIELD);
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
    let mut value = serde_json::to_value(&address).unwrap();
    value["socket"] = "peer.sock".into();
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
}
