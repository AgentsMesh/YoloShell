use super::*;

const PAGE_LIMIT: u16 = 1;

#[test]
fn reserve_is_idempotent_by_start_token() {
    let (_root, _hub, mut store) = fixture();
    let token = Uuid::new_v4();
    let first = reserve(&mut store, "alpha", token);
    let second = reserve(&mut store, "alpha", token);
    assert_eq!(first, second);
    assert_eq!(first.revision, FIRST_AGENT_RUN_REVISION);
    assert_eq!(first.state, AgentRunState::Starting);
    assert_eq!(store.resume(token).unwrap(), Some(first));
}

#[test]
fn reserve_rejects_duplicate_names_case_folded() {
    let (_root, _hub, mut store) = fixture();
    reserve(&mut store, "Alpha", Uuid::new_v4());
    let error = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "ALPHA".into(),
            provider_id: "codex".into(),
            workspace: None,
            parent: None,
            start_token: Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(error.into_public(), PublicError::Conflict));
}

#[test]
fn mark_launched_is_single_transition() {
    let (_root, _hub, mut store) = fixture();
    let run = reserve(&mut store, "alpha", Uuid::new_v4());
    let address = terminal();
    let launched = store.mark_launched(run.address, address).unwrap();
    assert_eq!(launched.state, AgentRunState::Launched);
    assert_eq!(launched.terminal, Some(address));
    assert_eq!(launched.revision, run.revision + 1);
    assert_eq!(store.mark_launched(run.address, address).unwrap(), launched);
    let other = terminal();
    let error = store.mark_launched(run.address, other).unwrap_err();
    assert!(matches!(error.into_public(), PublicError::Conflict));
}

#[test]
fn update_uses_revision_cas_and_alias_uniqueness() {
    let (_root, _hub, mut store) = fixture();
    let alpha = reserve(&mut store, "alpha", Uuid::new_v4());
    let beta = reserve(&mut store, "beta", Uuid::new_v4());
    let updated = update(&mut store, alpha.address, 1, Some("alpha2"), Some(vec!["a".into()]))
        .unwrap();
    assert_eq!(updated.revision, 2);
    assert_eq!(updated.name, "alpha2");
    assert_eq!(
        get(&store, AgentRunSelector::Name("A".into())).unwrap().address,
        alpha.address
    );
    let stale = update(&mut store, alpha.address, 1, Some("stale"), None).unwrap_err();
    assert!(matches!(stale.into_public(), PublicError::Conflict));
    let stolen = update(&mut store, beta.address, 1, None, Some(vec!["ALPHA2".into()])).unwrap_err();
    assert!(matches!(stolen.into_public(), PublicError::Conflict));
}

#[test]
fn store_reopens_and_keeps_state() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let mut store =
        AgentRunStore::open(yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(), hub)
            .unwrap();
    let run = reserve(&mut store, "alpha", Uuid::new_v4());
    store.mark_launched(run.address, terminal()).unwrap();
    drop(store);
    let reopened =
        AgentRunStore::open(yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(), hub)
            .unwrap();
    let loaded = reopened.get(&GetAgentRunRequest::new(GetAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        selector: AgentRunSelector::Address(run.address),
    }).unwrap()).unwrap().unwrap();
    assert_eq!(loaded.state, AgentRunState::Launched);
}

#[test]
fn list_paginates_and_filters() {
    let (_root, _hub, mut store) = fixture();
    let first = reserve(&mut store, "alpha", Uuid::new_v4());
    let second = reserve(&mut store, "beta", Uuid::new_v4());
    let address = terminal();
    store.mark_launched(second.address, address).unwrap();
    let page = |after, filter| {
        ListAgentRunsRequest::new(ListAgentRunsFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after,
            limit: PAGE_LIMIT,
            filter,
        })
        .unwrap()
    };
    let (rows, next) = store.list(&page(None, None)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(next, Some(rows[0].address));
    let (rows, next) = store.list(&page(next, None)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(next, None);
    let (rows, _) = store
        .list(&page(None, Some(AgentRunFilter::Terminal { address })))
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].address, second.address);
    let (rows, _) = store
        .list(&page(None, Some(AgentRunFilter::Name { value: "ALPHA".into() })))
        .unwrap();
    assert_eq!(rows[0].address, first.address);
}

#[test]
fn workspace_is_durable_and_part_of_reserve_idempotency() {
    let (_root, hub, mut store) = fixture();
    let workspace = WorkspaceAddress {
        hub_id: hub,
        workspace_id: yolo_hub_identity::WorkspaceId::from_uuid(Uuid::from_u128(5)).unwrap(),
    };
    let token = Uuid::new_v4();
    let summary = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "scoped".into(),
            provider_id: "codex".into(),
            workspace: Some(workspace),
            parent: None,
            start_token: token,
        })
        .unwrap();
    assert_eq!(summary.workspace, Some(workspace));
    let other = WorkspaceAddress {
        hub_id: hub,
        workspace_id: yolo_hub_identity::WorkspaceId::from_uuid(Uuid::from_u128(6)).unwrap(),
    };
    let error = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "scoped".into(),
            provider_id: "codex".into(),
            workspace: Some(other),
            parent: None,
            start_token: token,
        })
        .unwrap_err();
    assert!(matches!(error.into_public(), PublicError::Conflict));
}

#[test]
fn stop_requires_a_launched_terminal_and_is_idempotent() {
    let (_root, _hub, mut store) = fixture();
    let run = reserve(&mut store, "alpha", Uuid::new_v4());
    let error = store.mark_stopped(run.address).unwrap_err();
    assert!(matches!(error.into_public(), PublicError::Conflict));

    let launched = store.mark_launched(run.address, terminal()).unwrap();
    let stopped = store.mark_stopped(launched.address).unwrap();
    assert_eq!(stopped.state, AgentRunState::Stopped);
    assert!(stopped.revision > launched.revision);
    let again = store.mark_stopped(launched.address).unwrap();
    assert_eq!(again, stopped);
}
