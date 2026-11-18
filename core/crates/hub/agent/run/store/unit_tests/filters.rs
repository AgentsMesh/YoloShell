use super::*;

const FILTER_PAGE_LIMIT: u16 = 10;

#[test]
fn state_and_provider_filters_select_runs() {
    let (_root, _hub, mut store) = fixture();
    let first = reserve(&mut store, "alpha", Uuid::new_v4());
    let second = reserve(&mut store, "beta", Uuid::new_v4());
    store.mark_launched(first.address, terminal()).unwrap();
    let page = |filter| {
        ListAgentRunsRequest::new(ListAgentRunsFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit: FILTER_PAGE_LIMIT,
            filter: Some(filter),
        })
        .unwrap()
    };
    let (launched, _) = store
        .list(&page(AgentRunFilter::State {
            is: vec![AgentRunState::Launched],
        }))
        .unwrap();
    assert_eq!(
        launched.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![first.address]
    );
    let (starting, _) = store
        .list(&page(AgentRunFilter::State {
            is: vec![AgentRunState::Starting],
        }))
        .unwrap();
    assert_eq!(
        starting.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![second.address]
    );
    let (codex, _) = store
        .list(&page(AgentRunFilter::Provider { id: "CODEX".into() }))
        .unwrap();
    assert_eq!(codex.len(), 2);
    let (other, _) = store
        .list(&page(AgentRunFilter::Provider { id: "other".into() }))
        .unwrap();
    assert!(other.is_empty());
}

#[test]
fn label_filter_selects_runs_with_that_label() {
    let (_root, _hub, mut store) = fixture();
    let first = reserve(&mut store, "alpha", Uuid::new_v4());
    let second = reserve(&mut store, "beta", Uuid::new_v4());
    let request = UpdateAgentRunRequest::new(UpdateAgentRunFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        address: first.address,
        expected_revision: first.revision,
        name: None,
        aliases: None,
        labels: Some(
            [("team".into(), "alpha".into())]
                .into_iter()
                .collect(),
        ),
    })
    .unwrap();
    store.update(&request).unwrap();
    let page = |filter| {
        ListAgentRunsRequest::new(ListAgentRunsFields {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit: FILTER_PAGE_LIMIT,
            filter: Some(filter),
        })
        .unwrap()
    };
    let (labeled, _) = store
        .list(&page(AgentRunFilter::Label {
            key: "team".into(),
            value: "alpha".into(),
        }))
        .unwrap();
    assert_eq!(
        labeled.iter().map(|run| run.address).collect::<Vec<_>>(),
        vec![first.address]
    );
    let (missing, _) = store
        .list(&page(AgentRunFilter::Label {
            key: "team".into(),
            value: "beta".into(),
        }))
        .unwrap();
    assert!(missing.is_empty());
    assert_ne!(first.address, second.address);
}
