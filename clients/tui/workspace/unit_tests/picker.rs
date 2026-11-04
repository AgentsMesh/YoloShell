use crate::{Handle, Picker};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, WorkspaceAddress, WorkspaceId};
use yolo_hub_workspace_protocol::{
    ListWorkspacesResult, WorkspaceCapabilities, WorkspaceKind, WorkspaceSummary,
};

pub(crate) fn summary(address: WorkspaceAddress, name: &str) -> WorkspaceSummary {
    WorkspaceSummary {
        address,
        revision: 1,
        name: name.into(),
        root: format!("/tmp/{name}"),
        kind: WorkspaceKind::Directory,
        parent: None,
        labels: BTreeMap::new(),
        member_terminals: 0,
        member_runs: 0,
        capabilities: WorkspaceCapabilities::unsupported("test"),
        created_at_ms: 0,
        updated_at_ms: 0,
    }
}

pub(crate) fn address(hub: HubId) -> WorkspaceAddress {
    WorkspaceAddress {
        hub_id: hub,
        workspace_id: WorkspaceId::new(),
    }
}

pub(crate) fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

#[test]
fn paging_appends_rows_and_selects_the_current_filter() {
    let hub = HubId::new();
    let first = address(hub);
    let second = address(hub);
    let mut picker = Picker::default();
    assert!(picker.open(hub, Some(second), "/tmp/root".into()).is_some());
    assert!(picker.is_loading());
    let next = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(first, "first")],
        next: Some(first),
    });
    assert!(next.is_some());
    assert!(picker.page(ListWorkspacesResult {
        workspaces: vec![summary(second, "second")],
        next: None,
    })
    .is_none());
    assert!(!picker.is_loading());
    assert_eq!(picker.workspaces().len(), 2);
    assert_eq!(picker.selected(), 2);
    assert!(matches!(picker.key(&key(KeyCode::Up)), Handle::None));
    assert_eq!(picker.selected(), 1);
    match picker.key(&key(KeyCode::Enter)) {
        Handle::Filter(Some(workspace)) => assert_eq!(workspace, first),
        _ => panic!("expected the first workspace"),
    }
    assert!(!picker.is_open());
}

#[test]
fn the_first_row_clears_the_filter() {
    let hub = HubId::new();
    let first = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, Some(first), "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(first, "first")],
        next: None,
    });
    assert!(matches!(picker.key(&key(KeyCode::Home)), Handle::None));
    match picker.key(&key(KeyCode::Enter)) {
        Handle::Filter(None) => {}
        _ => panic!("expected the all-workspaces row"),
    }
}

#[test]
fn stale_pages_never_populate_a_reopened_picker() {
    let hub = HubId::new();
    let first = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    assert!(picker.open(hub, Some(first), "/tmp/root".into()).is_none());
    assert!(picker
        .page(ListWorkspacesResult {
            workspaces: vec![summary(first, "stale")],
            next: None,
        })
        .is_some());
    assert!(picker.workspaces().is_empty());
    assert!(picker
        .page(ListWorkspacesResult {
            workspaces: vec![summary(first, "fresh")],
            next: None,
        })
        .is_none());
    assert_eq!(picker.workspaces().len(), 1);
    assert_eq!(picker.selected(), 1);
}

#[test]
fn refresh_resets_rows_and_failure_reports_a_message() {
    let hub = HubId::new();
    let first = address(hub);
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    let _ = picker.page(ListWorkspacesResult {
        workspaces: vec![summary(first, "first")],
        next: None,
    });
    assert!(matches!(
        picker.key(&key(KeyCode::Char('r'))),
        Handle::Query(_)
    ));
    assert!(picker.workspaces().is_empty());
    picker.failed("listing failed");
    assert!(!picker.is_loading());
    assert_eq!(picker.message(), Some("listing failed"));
    assert!(matches!(picker.key(&key(KeyCode::Esc)), Handle::None));
    assert!(!picker.is_open());
}

#[test]
fn a_failed_stale_list_releases_the_reopen_guard() {
    let hub = HubId::new();
    let mut picker = Picker::default();
    let _ = picker.open(hub, None, "/tmp/root".into());
    assert!(picker.open(hub, None, "/tmp/root".into()).is_none());
    picker.failed("listing failed");
    assert!(matches!(
        picker.key(&key(KeyCode::Char('r'))),
        Handle::Query(_)
    ));
    assert!(picker
        .page(ListWorkspacesResult {
            workspaces: Vec::new(),
            next: None,
        })
        .is_none());
}
