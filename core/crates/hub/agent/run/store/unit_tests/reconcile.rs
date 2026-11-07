use super::*;

#[test]
fn reopen_keeps_terminal_less_starting_runs_resumable() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let mut store =
        AgentRunStore::open(yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(), hub)
            .unwrap();
    let starting = reserve(&mut store, "starting", Uuid::new_v4());
    let launched = reserve(&mut store, "launched", Uuid::new_v4());
    store.mark_launched(launched.address, terminal()).unwrap();
    drop(store);
    let reopened =
        AgentRunStore::open(yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(), hub)
            .unwrap();
    let get = |address| {
        reopened
            .get(
                &GetAgentRunRequest::new(GetAgentRunFields {
                    schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
                    selector: AgentRunSelector::Address(address),
                })
                .unwrap(),
            )
            .unwrap()
            .unwrap()
    };
    assert_eq!(get(starting.address).state, AgentRunState::Starting);
    assert_eq!(get(launched.address).state, AgentRunState::Launched);
}

