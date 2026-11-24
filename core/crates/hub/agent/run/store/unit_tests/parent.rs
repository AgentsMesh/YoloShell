use super::*;

const PARENT_PAGE_LIMIT: u16 = 10;

#[test]
fn parent_lineage_is_stored_and_filterable() {
    let (_root, hub, mut store) = fixture();
    let parent = reserve(&mut store, "parent", Uuid::new_v4());
    let child = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "child".into(),
            provider_id: "codex".into(),
            workspace: None,
            parent: Some(parent.address),
            start_token: Uuid::new_v4(),
        })
        .unwrap();
    assert_eq!(child.parent, Some(parent.address));
    let request = ListAgentRunsRequest::new(ListAgentRunsFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        after: None,
        limit: PARENT_PAGE_LIMIT,
        filter: Some(AgentRunFilter::Parent {
            address: parent.address,
        }),
    })
    .unwrap();
    let (rows, next) = store.list(&request).unwrap();
    assert_eq!(
        rows.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![child.address]
    );
    assert_eq!(next, None);

    let missing = AgentRunAddress {
        hub_id: hub,
        run_id: AgentRunId::new(),
    };
    let error = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "orphan".into(),
            provider_id: "codex".into(),
            workspace: None,
            parent: Some(missing),
            start_token: Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(error.into_public(), PublicError::NotFound));

    let foreign = AgentRunAddress {
        hub_id: HubId::new(),
        run_id: parent.address.run_id,
    };
    let error = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: "foreign".into(),
            provider_id: "codex".into(),
            workspace: None,
            parent: Some(foreign),
            start_token: Uuid::new_v4(),
        })
        .unwrap_err();
    assert!(matches!(error.into_public(), PublicError::OriginMismatch));
}
