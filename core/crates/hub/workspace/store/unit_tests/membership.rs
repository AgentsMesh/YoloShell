use super::*;

#[test]
fn terminal_and_run_members_are_counted_and_survive_reopen() {
    let (root, hub, mut store) = fixture();
    let created = store.create(&create(hub, Uuid::from_u128(1), "member")).unwrap();
    assert_eq!((created.member_terminals, created.member_runs), (0, 0));
    let address = created.address;
    let terminal = Uuid::from_u128(11);
    let run = Uuid::from_u128(12);
    store.bind_terminal(address, terminal).unwrap();
    store.bind_terminal(address, terminal).unwrap();
    store.bind_run(address, run).unwrap();
    let summary = store.get(address).unwrap().unwrap();
    assert_eq!((summary.member_terminals, summary.member_runs), (1, 1));
    assert_eq!(store.terminal_workspaces().unwrap(), vec![(terminal, address)]);
    assert_eq!(
        store.members_many(&[address]).unwrap(),
        vec![Some((vec![terminal], vec![run]))]
    );

    let path = root.path().canonicalize().unwrap().join("workspaces");
    drop(store);
    let reopened = WorkspaceStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    let summary = reopened.get(address).unwrap().unwrap();
    assert_eq!((summary.member_terminals, summary.member_runs), (1, 1));
}

#[test]
fn update_binds_and_unbinds_members_under_revision_cas() {
    let (_root, hub, mut store) = fixture();
    let first = store.create(&create(hub, Uuid::from_u128(1), "first")).unwrap();
    let second = store.create(&create(hub, Uuid::from_u128(2), "second")).unwrap();
    let terminal = Uuid::from_u128(31);
    let bind = UpdateWorkspaceRequest::new(UpdateWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: first.address,
        expected_revision: first.revision,
        name: None,
        labels: None,
        members: Some(MembershipChange::Bind {
            terminals: vec![terminal],
        }),
    })
    .unwrap();
    let bound = store.update(&bind).unwrap();
    assert_eq!((bound.member_terminals, bound.member_runs), (1, 0));
    assert_eq!(bound.revision, first.revision + 1);

    let conflict = UpdateWorkspaceRequest::new(UpdateWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: second.address,
        expected_revision: second.revision,
        name: None,
        labels: None,
        members: Some(MembershipChange::Bind {
            terminals: vec![terminal],
        }),
    })
    .unwrap();
    assert_eq!(
        store.update(&conflict).unwrap_err().into_public(),
        PublicError::Conflict
    );

    let unbind = UpdateWorkspaceRequest::new(UpdateWorkspaceFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: first.address,
        expected_revision: bound.revision,
        name: None,
        labels: None,
        members: Some(MembershipChange::Unbind {
            terminals: vec![terminal],
        }),
    })
    .unwrap();
    let unbound = store.update(&unbind).unwrap();
    assert_eq!((unbound.member_terminals, unbound.member_runs), (0, 0));
}

#[test]
fn a_member_belongs_to_one_workspace_and_needs_a_known_address() {
    let (_root, hub, mut store) = fixture();
    let first = store.create(&create(hub, Uuid::from_u128(1), "first")).unwrap();
    let second = store.create(&create(hub, Uuid::from_u128(2), "second")).unwrap();
    let terminal = Uuid::from_u128(21);
    store.bind_terminal(first.address, terminal).unwrap();
    assert_eq!(
        store
            .bind_terminal(second.address, terminal)
            .unwrap_err()
            .into_public(),
        PublicError::Conflict
    );
    let missing = WorkspaceAddress {
        hub_id: hub,
        workspace_id: yolo_hub_identity::WorkspaceId::from_uuid(Uuid::from_u128(3)).unwrap(),
    };
    assert_eq!(
        store.bind_run(missing, Uuid::from_u128(22)).unwrap_err().into_public(),
        PublicError::NotFound
    );
    assert_eq!(
        store.bind_run(first.address, Uuid::nil()).unwrap_err().into_public(),
        PublicError::InvalidRequest
    );
}
