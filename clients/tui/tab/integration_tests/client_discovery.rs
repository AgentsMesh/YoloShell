use crate::support::*;
use serde_json::json;
use std::os::unix::fs::symlink;
use uuid::Uuid;
use yolo_paths::directory::PrivateDirectory;
use yolo_tui_tab::{
    DEFAULT_TAB_NAME, FIRST_TAB_REVISION, TAB_FILE, TAB_SCHEMA_VERSION,
    TabStore, client_directories_in, open_client_in,
};

#[test]
fn missing_state_discovery_is_read_only() {
    let (_root, base, hub) = fixture();
    assert!(client_directories_in(&base, hub).unwrap().is_empty());
    assert!(!base.exists());
}

#[test]
fn unreadable_client_state_is_preserved_while_a_fresh_client_is_allocated() {
    let (_root, base, hub) = fixture();
    drop(TabStore::open(&base, hub).unwrap());
    let stale = json!({
        SCHEMA_FIELD: TAB_SCHEMA_VERSION,
        ENTRY_FIELD: hub,
        REVISION_FIELD: FIRST_TAB_REVISION,
        TABS_FIELD: {
            ACTIVE_FIELD: Uuid::new_v4(),
            ENTRIES_FIELD: [{
                ID_FIELD: Uuid::new_v4(),
                NAME_FIELD: DEFAULT_TAB_NAME,
                RETIRED_WORKSPACE_FIELD: {
                    LAYOUT_FIELD: {
                        ROOT_FIELD: {PANE_FIELD: {
                            ID_FIELD: Uuid::new_v4(),
                            SESSION_FIELD: {HUB_ID_FIELD: hub, SESSION_ID_FIELD: Uuid::new_v4()}
                        }},
                        FOCUSED_FIELD: Uuid::new_v4()
                    },
                    VIEW_FIELD: TERMINAL_VIEW
                }
            }]
        }
    });
    let path = base.join(TAB_FILE);
    yolo_paths::atomic_write_json(&path, &stale).unwrap();
    let before = std::fs::read(&path).unwrap();

    let fresh = open_client_in(&base, hub).unwrap();
    assert!(
        fresh
            .directory()
            .unwrap()
            .path()
            .starts_with(base.join(CLIENTS_DIRECTORY))
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn discovery_ignores_unknown_and_indirect_entries_without_following_or_deleting_them() {
    let (root, base, hub) = fixture();
    let _owner = open_client_in(&base, hub).unwrap();
    let clients = PrivateDirectory::ensure(&base.join(CLIENTS_DIRECTORY)).unwrap();
    let accepted = clients.path().join(FIRST_CHILD_ID);
    let _accepted_owner = TabStore::open(&accepted, hub).unwrap();
    let outside = root.path().join(REPLACEMENT);
    PrivateDirectory::ensure(&outside).unwrap();
    let unknown = clients.path().join(UNKNOWN_CLIENT_ENTRY);
    std::fs::write(&unknown, INVALID_BYTES).unwrap();
    let symlink_path = clients.path().join(SECOND_CHILD_ID);
    symlink(&outside, &symlink_path).unwrap();
    let nil = clients.path().join(Uuid::nil().to_string());
    clients.ensure_descendant(&nil).unwrap();
    let uppercase = clients.path().join(FIRST_CHILD_ID.to_uppercase());
    // Case-insensitive systems may alias this name; the accepted lowercase entry is sufficient.
    if !uppercase.exists() {
        clients.ensure_descendant(&uppercase).unwrap();
    }
    let result = client_directories_in(&base, hub).unwrap();
    assert_eq!(
        result
            .iter()
            .filter(|client| client.path != base)
            .map(|client| &client.path)
            .collect::<Vec<_>>(),
        vec![&accepted]
    );
    assert_eq!(std::fs::read(unknown).unwrap(), INVALID_BYTES);
    assert!(
        std::fs::symlink_metadata(symlink_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(nil.exists());
    assert!(outside.exists());
}

#[test]
fn an_indirect_clients_root_or_default_root_is_never_traversed() {
    let (root, base, hub) = fixture();
    let _owner = open_client_in(&base, hub).unwrap();
    let outside = root.path().join(REPLACEMENT);
    PrivateDirectory::ensure(&outside).unwrap();
    symlink(&outside, base.join(CLIENTS_DIRECTORY)).unwrap();
    assert!(client_directories_in(&base, hub).is_err());
    let alias = root.path().join(UNKNOWN_CLIENT_ENTRY);
    symlink(&base, &alias).unwrap();
    assert!(client_directories_in(&alias, hub).is_err());
}

#[test]
fn direct_override_cannot_discover_or_relabel_a_different_hubs_client() {
    let (_root, base, hub) = fixture();
    let _owner = open_client_in(&base, hub).unwrap();
    let before = std::fs::read(base.join(TAB_FILE)).unwrap();
    assert!(client_directories_in(&base, yolo_hub_identity::HubId::new()).is_err());
    assert_eq!(std::fs::read(base.join(TAB_FILE)).unwrap(), before);
    let child_path = base.join(CLIENTS_DIRECTORY).join(FIRST_CHILD_ID);
    let _foreign = TabStore::open(&child_path, yolo_hub_identity::HubId::new()).unwrap();
    let child_before = std::fs::read(child_path.join(TAB_FILE)).unwrap();
    assert!(client_directories_in(&base, hub).is_err());
    assert_eq!(
        std::fs::read(child_path.join(TAB_FILE)).unwrap(),
        child_before
    );
}
