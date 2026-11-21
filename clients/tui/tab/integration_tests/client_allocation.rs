use crate::support::*;
use yolo_tui_tab::{TabStore, client_directories_in, open_client_in};

#[test]
fn concurrent_clients_keep_independent_state_and_reclaim_an_inactive_owner() {
    let (_root, base, hub) = fixture();
    let first = open_client_in(&base, hub).unwrap();
    let second = open_client_in(&base, hub).unwrap();
    let third = open_client_in(&base, hub).unwrap();
    let first_path = first.directory().unwrap().path().to_owned();
    let second_path = second.directory().unwrap().path().to_owned();
    let third_path = third.directory().unwrap().path().to_owned();
    assert_eq!(first_path, base);
    assert_ne!(first_path, second_path);
    assert_ne!(second_path, third_path);
    assert!(second_path.starts_with(base.join(CLIENTS_DIRECTORY)));
    assert!(third_path.starts_with(base.join(CLIENTS_DIRECTORY)));
    assert!(TabStore::try_open(&base, hub).unwrap().is_none());
    let clients = client_directories_in(&base, hub).unwrap();
    assert_eq!(clients.len(), CLIENT_DIRECTORY_COUNT);
    assert_eq!(clients.first().unwrap().id, hub.as_uuid());
    assert!(clients.iter().any(|client| client.path == second_path
        && client.id.to_string() == second_path.file_name().unwrap().to_str().unwrap()));
    drop(second);
    let replacement = open_client_in(&base, hub).unwrap();
    assert_eq!(replacement.directory().unwrap().path(), second_path);
    first.directory().unwrap().verify_path().unwrap();
    third.directory().unwrap().verify_path().unwrap();
}

#[test]
fn closed_default_state_is_reused_without_allocating_a_new_directory() {
    let (_root, base, hub) = fixture();
    let first = open_client_in(&base, hub).unwrap();
    let expected = first.document().clone();
    drop(first);
    let replacement = open_client_in(&base, hub).unwrap();
    assert_eq!(replacement.directory().unwrap().path(), base);
    assert_eq!(replacement.document(), &expected);
    assert!(!base.join(CLIENTS_DIRECTORY).exists());
}
