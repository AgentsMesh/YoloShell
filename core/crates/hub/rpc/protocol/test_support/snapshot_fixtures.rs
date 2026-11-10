use crate::fixtures::{constants::*, *};
use yolo_hub_rpc_protocol::{
    Command, GetPlatformSnapshotFields, GetPlatformSnapshotRequest,
    PLATFORM_SNAPSHOT_SCHEMA_VERSION, PlatformSnapshot, PlatformSnapshotFields,
    PlatformSnapshotTruncation, Reply,
};

const SECTION_LIMIT: u16 = 1;

pub fn samples() -> Vec<(Command, Reply)> {
    vec![(
        Command::GetPlatformSnapshot {
            request: GetPlatformSnapshotRequest::new(GetPlatformSnapshotFields {
                schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
                terminal_limit: SECTION_LIMIT,
                agent_limit: SECTION_LIMIT,
                run_limit: SECTION_LIMIT,
                workspace_limit: SECTION_LIMIT,
            })
            .unwrap(),
        },
        Reply::PlatformSnapshot {
            snapshot: PlatformSnapshot::new(PlatformSnapshotFields {
                schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
                generated_at_ms: 0,
                hubs: vec![yolo_hub_rpc_protocol::HubSummary {
                    hub_id: hub(HUB_ID),
                    reachable: true,
                    label: None,
                }],
                terminals: vec![summary(HUB_ID, SESSION_ID)],
                agents: vec![agent()],
                runs: vec![agent_run_summary()],
                workspaces: vec![workspace_summary()],
                truncated: PlatformSnapshotTruncation::default(),
            })
            .unwrap(),
        },
    )]
}
