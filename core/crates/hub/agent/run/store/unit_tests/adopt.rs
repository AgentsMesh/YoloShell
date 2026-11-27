use super::*;

const PAGE_LIMIT: u16 = 16;

fn observed(hub: HubId) -> TerminalAddress {
    TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::new(),
    }
}

fn runs(store: &AgentRunStore) -> Vec<AgentRunSummary> {
    let request = ListAgentRunsRequest::new(ListAgentRunsFields {
        schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
        after: None,
        limit: PAGE_LIMIT,
        filter: None,
    })
    .unwrap();
    store.list(&request).unwrap().0
}

#[test]
fn adopt_materializes_one_launched_run_per_terminal() {
    let (_root, hub, mut store) = fixture();
    let terminal = observed(hub);
    let run = store.adopt(terminal, "codex").unwrap().unwrap();
    assert_eq!(run.state, AgentRunState::Launched);
    assert_eq!(run.terminal, Some(terminal));
    assert_eq!(run.provider_id, "codex");
    assert_eq!(
        run.name,
        format!("observed-{}", terminal.terminal_id.as_uuid().simple())
    );
    assert_eq!(run.revision, FIRST_AGENT_RUN_REVISION);
    // A repeated observation of the same execution is not a second run.
    assert_eq!(store.adopt(terminal, "codex").unwrap(), None);
    assert_eq!(runs(&store), vec![run]);
}

#[test]
fn adopt_skips_invalid_foreign_and_covered_terminals() {
    let (_root, hub, mut store) = fixture();
    let terminal = observed(hub);
    assert_eq!(store.adopt(terminal, "Not A Provider").unwrap(), None);
    let foreign = observed(HubId::new());
    assert_eq!(store.adopt(foreign, "codex").unwrap(), None);
    assert!(runs(&store).is_empty());

    let reserved = reserve(&mut store, "alpha", Uuid::new_v4());
    let launched = store.mark_launched(reserved.address, terminal).unwrap();
    assert_eq!(store.adopt(terminal, "codex").unwrap(), None);
    assert_eq!(runs(&store), vec![launched]);
}
