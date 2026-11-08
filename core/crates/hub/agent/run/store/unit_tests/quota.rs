use super::*;

fn reserve_with(
    store: &mut AgentRunStore,
    name: &str,
    parent: Option<AgentRunAddress>,
) -> Result<AgentRunSummary, PublicError> {
    store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: name.into(),
            provider_id: "codex".into(),
            workspace: None,
            parent,
            start_token: Uuid::new_v4(),
        })
        .map_err(StoreError::into_public)
}

fn launch(store: &mut AgentRunStore, run: AgentRunSummary) -> AgentRunSummary {
    store.mark_launched(run.address, terminal()).unwrap()
}

#[test]
fn active_run_cap_rejects_new_runs_and_frees_after_stop() {
    let (_root, _hub, mut store) = fixture();
    let mut runs = Vec::new();
    for index in 0..MAX_ACTIVE_AGENT_RUNS {
        let run = reserve_with(&mut store, &format!("run-{index}"), None).unwrap();
        runs.push(launch(&mut store, run));
    }
    assert!(matches!(
        reserve_with(&mut store, "overflow", None).unwrap_err(),
        PublicError::LimitExceeded
    ));

    // An idempotent retry of an existing reservation still returns its run.
    let existing = store
        .reserve(ReserveAgentRun {
            run_id: Uuid::new_v4(),
            name: runs[0].name.clone(),
            provider_id: "codex".into(),
            workspace: None,
            parent: None,
            start_token: runs[0].start_token,
        })
        .unwrap();
    assert_eq!(existing.address, runs[0].address);

    store.mark_stopped(runs[0].address).unwrap();
    reserve_with(&mut store, "after-stop", None).unwrap();
}

#[test]
fn cancelled_starting_run_frees_its_quota_slot() {
    let (_root, _hub, mut store) = fixture();
    let starting: Vec<_> = (0..MAX_ACTIVE_AGENT_RUNS)
        .map(|index| reserve_with(&mut store, &format!("starting-{index}"), None).unwrap())
        .collect();
    assert!(matches!(
        reserve_with(&mut store, "overflow", None).unwrap_err(),
        PublicError::LimitExceeded
    ));
    store.cancel_starting(starting[0].address).unwrap();
    reserve_with(&mut store, "after-cancel", None).unwrap();
    store.cancel_starting(starting[0].address).unwrap();
}

#[test]
fn retried_starting_runs_count_against_quota_after_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let open = || {
        AgentRunStore::open(
            yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
            hub,
        )
        .unwrap()
    };
    let mut store = open();
    let mut tokens = Vec::new();
    for index in 0..MAX_ACTIVE_AGENT_RUNS {
        let token = Uuid::new_v4();
        store
            .reserve(ReserveAgentRun {
                run_id: Uuid::new_v4(),
                name: format!("start-{index}"),
                provider_id: "codex".into(),
                workspace: None,
                parent: None,
                start_token: token,
            })
            .unwrap();
        tokens.push(token);
    }
    drop(store);
    let mut store = open();
    for token in &tokens {
        let run = store.resume(*token).unwrap().unwrap();
        store.rejoin_starting(run.address).unwrap();
    }
    assert!(matches!(
        reserve_with(&mut store, "overflow", None).unwrap_err(),
        PublicError::LimitExceeded
    ));
}

#[test]
fn parent_fan_out_and_lineage_depth_are_bounded() {
    let (_root, _hub, mut store) = fixture();
    let root = reserve_with(&mut store, "root", None).unwrap();
    let root = launch(&mut store, root);
    let mut children = Vec::new();
    for index in 0..MAX_AGENT_RUN_CHILDREN {
        let name = format!("child-{index}");
        let child = reserve_with(&mut store, &name, Some(root.address)).unwrap();
        children.push(launch(&mut store, child));
    }
    assert!(matches!(
        reserve_with(&mut store, "too-many", Some(root.address)).unwrap_err(),
        PublicError::LimitExceeded
    ));
    store.mark_stopped(children[0].address).unwrap();
    reserve_with(&mut store, "replacement", Some(root.address)).unwrap();

    let mut parent = reserve_with(&mut store, "chain-0", None).unwrap();
    for depth in 1..=MAX_AGENT_RUN_DEPTH {
        let name = format!("chain-{depth}");
        parent = reserve_with(&mut store, &name, Some(parent.address)).unwrap();
    }
    assert!(matches!(
        reserve_with(&mut store, "chain-too-deep", Some(parent.address)).unwrap_err(),
        PublicError::LimitExceeded
    ));
}
