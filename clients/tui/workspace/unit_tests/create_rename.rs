use super::picker::{address, key, summary};
use crate::{Handle, Picker};
use crossterm::event::KeyCode;
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, WorkspaceId};
use yolo_hub_workspace_protocol::ListWorkspacesResult;

#[test]
fn creating_uses_the_root_and_matches_only_its_own_reply() {
    let hub = HubId::new();
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/checkout".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: Vec::new(),
        next: None,
    });
    assert!(matches!(picker.key(&key(KeyCode::Char('c'))), Handle::None));
    assert_eq!(picker.draft().map(|draft| draft.text.as_str()), Some("checkout"));
    let request = match picker.key(&key(KeyCode::Enter)) {
        Handle::Create(request) => request,
        _ => panic!("expected a create request"),
    };
    assert_eq!(request.fields().name, "checkout");
    assert_eq!(request.fields().root, "/tmp/checkout");
    assert!(picker.is_pending());

    let mut foreign = summary(address(hub), "foreign");
    foreign.address.workspace_id = WorkspaceId::new();
    assert!(!picker.created(foreign));
    assert!(picker.is_pending());

    let mut created = summary(address(hub), "checkout");
    created.address.workspace_id = WorkspaceId::from_uuid(request.fields().workspace_id).unwrap();
    assert!(picker.created(created));
    assert!(!picker.is_pending());
    assert_eq!(picker.workspaces().len(), 1);
    assert_eq!(picker.message(), Some("Created checkout"));
}

#[test]
fn a_failed_creation_keeps_the_draft_for_retry() {
    let hub = HubId::new();
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/checkout".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: Vec::new(),
        next: None,
    });
    let _ = picker.key(&key(KeyCode::Char('c')));
    let request = match picker.key(&key(KeyCode::Enter)) {
        Handle::Create(request) => request,
        _ => panic!("expected a create request"),
    };
    picker.create_failed(request.fields().workspace_id, "root rejected");
    assert!(!picker.is_pending());
    assert!(picker.draft().is_some());
    assert_eq!(picker.message(), Some("root rejected"));
}

#[test]
fn renaming_uses_the_row_revision_and_matches_only_its_own_reply() {
    let hub = HubId::new();
    let workspace = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(workspace, "old")],
        next: None,
    });
    let _ = picker.key(&key(KeyCode::Down));
    let _ = picker.key(&key(KeyCode::Char('e')));
    assert_eq!(picker.draft().map(|draft| draft.text.as_str()), Some("old"));
    let request = match picker.key(&key(KeyCode::Enter)) {
        Handle::Rename(request) => request,
        _ => panic!("expected a rename request"),
    };
    assert_eq!(request.fields().address, workspace);
    assert_eq!(request.fields().expected_revision, 1);
    assert_eq!(request.fields().name.as_deref(), Some("old"));
    assert!(picker.is_pending());

    let mut updated = summary(workspace, "new");
    updated.revision = 2;
    assert!(picker.updated(updated));
    assert!(!picker.is_pending());
    assert_eq!(picker.workspaces()[0].name, "new");
    assert_eq!(picker.message(), Some("Renamed new"));
}

#[test]
fn a_failed_rename_keeps_the_draft_for_retry() {
    let hub = HubId::new();
    let workspace = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(workspace, "old")],
        next: None,
    });
    let _ = picker.key(&key(KeyCode::Down));
    let _ = picker.key(&key(KeyCode::Char('e')));
    let _ = picker.key(&key(KeyCode::Enter));
    picker.update_failed(workspace, "revision changed");
    assert!(!picker.is_pending());
    assert!(picker.draft().is_some());
    assert_eq!(picker.message(), Some("revision changed"));
}

#[test]
fn labels_are_prefilled_parsed_and_saved() {
    let hub = HubId::new();
    let workspace = address(hub);
    let mut row = summary(workspace, "labeled");
    row.labels.insert("team".into(), "core".into());
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![row],
        next: None,
    });
    let _ = picker.key(&key(KeyCode::Down));
    let _ = picker.key(&key(KeyCode::Char('l')));
    assert_eq!(picker.draft().map(|draft| draft.text.as_str()), Some("team=core"));
    for character in "team=platform,env=prod".chars() {
        picker.key(&key(KeyCode::Char(character)));
    }
    let request = match picker.key(&key(KeyCode::Enter)) {
        Handle::Rename(request) => request,
        _ => panic!("expected an update request"),
    };
    assert_eq!(request.fields().name, None);
    let expected = BTreeMap::from([
        ("team".into(), "platform".into()),
        ("env".into(), "prod".into()),
    ]);
    assert_eq!(request.fields().labels, Some(expected));

    let mut updated = summary(workspace, "labeled");
    updated.revision = 2;
    updated.labels = request.fields().labels.clone().unwrap();
    assert!(picker.updated(updated));
    assert_eq!(
        picker.workspaces()[0].labels.get("team"),
        Some(&"platform".into())
    );
    assert_eq!(picker.message(), Some("Labels updated"));
}

#[test]
fn invalid_label_text_keeps_the_draft() {
    let hub = HubId::new();
    let workspace = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(workspace, "labeled")],
        next: None,
    });
    let _ = picker.key(&key(KeyCode::Down));
    let _ = picker.key(&key(KeyCode::Char('l')));
    picker.key(&key(KeyCode::Char('x')));
    assert!(matches!(picker.key(&key(KeyCode::Enter)), Handle::None));
    assert!(picker.draft().is_some());
    assert_eq!(picker.message(), Some("Enter labels as key=value pairs"));
}
