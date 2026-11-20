use super::*;
use std::collections::BTreeMap;

#[test]
fn observed_exit_releases_quota_and_only_a_replacement_runtime_reopens_the_run() {
    let (root, hub, mut store) = fixture();
    let terminal = TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::new(),
    };
    let awaiting = TerminalAddress {
        terminal_id: TerminalId::new(),
        ..terminal
    };
    let original = store.adopt(terminal, "codex").unwrap().unwrap();
    let reserved = reserve(&mut store, "awaiting", Uuid::new_v4());
    let awaiting_run = store.mark_launched(reserved.address, awaiting).unwrap();
    let selector = AgentRunSelector::Address(original.address);
    let epoch = Uuid::new_v4();
    let ended = BTreeMap::from([
        (terminal, TerminalObservation::Ended(epoch)),
        (awaiting, TerminalObservation::Live(Uuid::new_v4())),
    ]);

    store
        .observe_terminals(&BTreeMap::from([(
            terminal,
            TerminalObservation::Live(epoch),
        )]))
        .unwrap();
    store.observe_terminals(&ended).unwrap();
    let stopped = get(&store, selector.clone()).unwrap();
    assert_eq!(stopped.state, AgentRunState::Stopped);
    assert_eq!(stopped.revision, original.revision + 2);
    assert_eq!(store.launched_terminals().unwrap(), [awaiting].into());
    assert!(store.mark_launched(original.address, terminal).is_err());
    let awaiting_after_observation =
        get(&store, AgentRunSelector::Address(awaiting_run.address)).unwrap();
    assert_eq!(awaiting_after_observation.address, awaiting_run.address);
    assert_eq!(awaiting_after_observation.state, AgentRunState::Launched);
    assert_eq!(
        awaiting_after_observation.revision,
        awaiting_run.revision + 1
    );
    store.observe_terminals(&ended).unwrap();
    assert_eq!(get(&store, selector.clone()), Some(stopped.clone()));

    drop(store);
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let mut store = AgentRunStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    let observe = |store: &mut AgentRunStore, report| {
        store
            .observe_terminals(&BTreeMap::from([(terminal, report)]))
            .unwrap();
        get(store, selector.clone()).unwrap()
    };
    assert_eq!(
        observe(&mut store, TerminalObservation::Live(epoch)),
        stopped
    );
    let replacement = observe(&mut store, TerminalObservation::Live(Uuid::new_v4()));
    assert_eq!(replacement.address, original.address);
    assert_eq!(replacement.state, AgentRunState::Launched);
    assert_eq!(replacement.revision, stopped.revision + 1);
    let current_epoch = get(&store, selector.clone()).unwrap();
    store
        .observe_terminals(&BTreeMap::from([(
            terminal,
            TerminalObservation::Ended(epoch),
        )]))
        .unwrap();
    assert_eq!(get(&store, selector.clone()).unwrap(), current_epoch);
    let archived = observe(&mut store, TerminalObservation::Archived);
    assert_eq!(archived.state, AgentRunState::Stopped);
    let after_archive = observe(&mut store, TerminalObservation::Live(Uuid::new_v4()));
    assert_eq!(after_archive.state, AgentRunState::Stopped);
    assert_eq!(after_archive.revision, archived.revision);
}

#[test]
fn reopen_keeps_terminal_less_starting_runs_resumable() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let mut store = AgentRunStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    let starting = reserve(&mut store, "starting", Uuid::new_v4());
    let launched = reserve(&mut store, "launched", Uuid::new_v4());
    store.mark_launched(launched.address, terminal()).unwrap();
    drop(store);
    let reopened = AgentRunStore::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
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
