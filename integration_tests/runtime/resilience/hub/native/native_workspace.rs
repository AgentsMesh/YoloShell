use super::{
    native_workspace_support::*,
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
};
use yolo_hub_identity::{HubId, RuntimeEpoch, TerminalId};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    entry_hub: HubId,
    terminal_id: TerminalId,
    runtime_epoch: RuntimeEpoch,
    archived: bool,
    marker_visible: bool,
    focus_unchanged: bool,
}

#[tokio::test(flavor = "multi_thread")]
async fn real_native_workspace_uses_public_hub_creation_terminal_and_lifecycle_without_focus() {
    let harness = crate::Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let root = root.path().canonicalize().unwrap();
    let state = root.join(STATE_DIRECTORY);
    let hub = RunningHub::start(&harness, &state).await;
    run(&hub, &harness, &root).await;
    let report: Report =
        serde_json::from_slice(&std::fs::read(root.join(REPORT_FILE)).unwrap()).unwrap();
    assert_eq!(report.entry_hub, hub.hub);
    assert!(report.archived && report.marker_visible && report.focus_unchanged);
    let session = harness
        .client()
        .get_session(report.terminal_id.as_uuid())
        .await
        .unwrap();
    assert!(session.manifest.archived);
    if let Some(runtime) = session.runtime {
        assert_eq!(runtime.runtime_epoch, report.runtime_epoch.as_uuid());
    }
    let workspace: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(WORKSPACE_FILE)).unwrap()).unwrap();
    assert_eq!(workspace[STORED_SCHEMA_FIELD], WORKSPACE_SCHEMA);
    assert_eq!(
        workspace[STATE_FIELD][STORED_HUB_FIELD]
            .as_str()
            .unwrap()
            .to_lowercase(),
        hub.hub.to_string()
    );
    assert_eq!(workspace[STATE_FIELD][PREFERRED_FIELD], hub.hub.to_string());
    hub.stop().await;
}
