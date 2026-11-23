use super::*;

const WORKTREE_PAGE_LIMIT: u16 = 16;

#[test]
fn create_is_idempotent_and_names_are_case_folded() {
    let (_root, hub, mut store) = fixture();
    let id = Uuid::new_v4();
    let first = store.create(&create(hub, id, "Alpha")).unwrap();
    assert_eq!(first.revision, 1);
    let retry = store.create(&create(hub, id, "Alpha")).unwrap();
    assert_eq!(retry, first);
    let conflict = store.create(&create(hub, Uuid::new_v4(), "alpha"));
    assert_eq!(conflict.unwrap_err().into_public(), PublicError::Conflict);
}

#[test]
fn update_uses_revision_cas_and_rejects_foreign_names() {
    let (_root, hub, mut store) = fixture();
    let alpha = store.create(&create(hub, Uuid::new_v4(), "Alpha")).unwrap();
    let beta = store.create(&create(hub, Uuid::new_v4(), "Beta")).unwrap();
    let rename = UpdateWorkspaceRequest::new(yolo_hub_workspace_protocol::UpdateWorkspaceFields {
        schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: alpha.address,
        expected_revision: alpha.revision,
        name: Some("Alpha Two".into()),
        labels: None,
        members: None,
    })
    .unwrap();
    let updated = store.update(&rename).unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.name, "Alpha Two");
    let stale = store.update(&rename);
    assert_eq!(stale.unwrap_err().into_public(), PublicError::Conflict);
    let mut taken = rename.fields().clone();
    taken.request_id = Uuid::new_v4();
    taken.expected_revision = updated.revision;
    taken.name = Some("beta".into());
    let taken = UpdateWorkspaceRequest::new(taken).unwrap();
    assert_eq!(
        store.update(&taken).unwrap_err().into_public(),
        PublicError::Conflict
    );
    assert_eq!(store.get(beta.address).unwrap(), Some(beta));
}

#[test]
fn reopening_preserves_rows_and_listing_is_cursor_bounded() {
    let (root, hub, mut store) = fixture();
    let addresses: Vec<_> = (0..3)
        .map(|index| {
            store
                .create(&create(hub, Uuid::new_v4(), &format!("ws-{index}")))
                .unwrap()
                .address
        })
        .collect();
    let (first, next) = store.list(&list(2, None)).unwrap();
    assert_eq!(first.len(), 2);
    assert!(next.is_some());
    let (second, next) = store.list(&list(2, next)).unwrap();
    assert_eq!(second.len(), 1);
    assert!(next.is_none());
    drop(store);
    let reopened = WorkspaceStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(
            &root.path().canonicalize().unwrap().join("workspaces"),
        )
        .unwrap(),
        hub,
    )
    .unwrap();
    let (all, _) = reopened.list(&list(8, None)).unwrap();
    let mut expected = addresses;
    expected.sort();
    assert_eq!(
        all.iter()
            .map(|workspace| workspace.address)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn worktrees_are_children_and_remove_keeps_the_parent() {
    let (_root, _hub, mut store) = fixture();
    let parent = store
        .create(&create(
            yolo_hub_identity::HubId::new(),
            Uuid::new_v4(),
            "parent",
        ))
        .unwrap();
    let request = yolo_hub_workspace_protocol::CreateWorktreeRequest::new(
        yolo_hub_workspace_protocol::CreateWorktreeFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            workspace_id: Uuid::new_v4(),
            parent: parent.address,
            name: "child".into(),
            branch: None,
            base: None,
            trust_repository: true,
        },
    )
    .unwrap();
    let child = store
        .create_worktree(&request, "/tmp/child".into())
        .unwrap();
    assert_eq!(child.kind, yolo_hub_workspace_protocol::WorkspaceKind::Worktree);
    assert_eq!(child.parent, Some(parent.address));
    let list = yolo_hub_workspace_protocol::ListWorktreesRequest::new(
        yolo_hub_workspace_protocol::ListWorktreesFields {
            schema_version: yolo_hub_workspace_protocol::WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            parent: parent.address,
            after: None,
            limit: WORKTREE_PAGE_LIMIT,
        },
    )
    .unwrap();
    assert_eq!(store.list_worktrees(&list).unwrap().0.len(), 1);

    let reservation = store.reserve_creation(child.address).unwrap();
    assert!(matches!(
        store.ensure_removable(child.address),
        Err(crate::StoreError::Protocol(PublicError::Conflict))
    ));
    drop(reservation);
    store.remove(child.address).unwrap();
    assert!(store.get(parent.address).unwrap().is_some());
    assert_eq!(store.list_worktrees(&list).unwrap().0.len(), 0);
}
