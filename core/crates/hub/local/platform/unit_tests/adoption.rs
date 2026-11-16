use crate::agent_run::AgentRunService;
use crate::snapshot::adopt_observed;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, Activity, AgentCapabilities, AgentSnapshot, AgentSnapshotFields,
    FIRST_OBSERVATION_SEQUENCE, Freshness, Lifecycle, ObservationCursor, ObservationSource,
    ObservedProvider, Reachability, RecoveryStatus,
};
use yolo_hub_agent_run_protocol::AgentRunState;
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, HubId, RuntimeEpoch, RuntimeTarget,
    TerminalAddress, TerminalId,
};

fn terminal(hub: HubId) -> TerminalAddress {
    TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::new(),
    }
}

fn snapshot(
    hub: HubId,
    terminal: TerminalAddress,
    freshness: Freshness,
    lifecycle: Lifecycle,
) -> AgentSnapshot {
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: hub,
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal: RuntimeTarget {
            address: terminal,
            runtime_epoch: RuntimeEpoch::new(),
        },
        provider: ObservedProvider::new("codex".into(), None, None).unwrap(),
        working_directory: None,
        lifecycle,
        activity: Activity::Idle,
        reachability: Reachability::Reachable,
        freshness,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: 1,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap()
}

fn service() -> (tempfile::TempDir, HubId, AgentRunService) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("agent-runs");
    let hub = HubId::new();
    let service = AgentRunService::open(
        yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap(),
        hub,
    )
    .unwrap();
    (root, hub, service)
}

#[tokio::test]
async fn adopt_observed_skips_exited_stale_and_repeated_executions() {
    let (_root, hub, runs) = service();
    let live = terminal(hub);
    let exited = terminal(hub);
    let stale = terminal(hub);
    let agents = vec![
        snapshot(hub, live, Freshness::Current, Lifecycle::Running {}),
        snapshot(
            hub,
            exited,
            Freshness::Current,
            Lifecycle::Exited { exit_code: Some(0) },
        ),
        snapshot(hub, stale, Freshness::Stale, Lifecycle::Running {}),
        snapshot(hub, live, Freshness::Current, Lifecycle::Running {}),
    ];
    adopt_observed(Some(&runs), &agents).await;
    let (page, more) = runs.snapshot_page(10).await.unwrap();
    assert!(!more);
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].terminal, Some(live));
    assert_eq!(page[0].provider_id, "codex");
    assert_eq!(page[0].state, AgentRunState::Launched);
}
