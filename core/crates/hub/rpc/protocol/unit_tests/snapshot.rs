use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::*;

const LIMIT: u16 = 8;

fn fields() -> GetPlatformSnapshotFields {
    GetPlatformSnapshotFields {
        schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
        terminal_limit: LIMIT,
        agent_limit: LIMIT,
        run_limit: LIMIT,
        workspace_limit: LIMIT,
    }
}

#[test]
fn snapshot_limits_are_non_zero_and_bounded() {
    assert!(GetPlatformSnapshotRequest::new(fields()).is_ok());
    for invalid in [0, MAX_PLATFORM_SNAPSHOT_SECTION + 1] {
        for index in 0..4 {
            let mut fields = fields();
            match index {
                0 => fields.terminal_limit = invalid,
                1 => fields.agent_limit = invalid,
                2 => fields.run_limit = invalid,
                _ => fields.workspace_limit = invalid,
            }
            assert_eq!(
                GetPlatformSnapshotRequest::new(fields),
                Err(yolo_hub_terminal_protocol::PublicError::InvalidRequest)
            );
        }
    }
    let mut fields = fields();
    fields.schema_version += 1;
    assert_eq!(
        GetPlatformSnapshotRequest::new(fields),
        Err(yolo_hub_terminal_protocol::PublicError::UnsupportedSchema)
    );
}

#[test]
fn snapshot_document_rejects_invalid_time_and_oversized_sections() {
    let mut fields = PlatformSnapshotFields {
        schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
        generated_at_ms: 0,
        hubs: Vec::new(),
        terminals: Vec::new(),
        agents: Vec::new(),
        runs: Vec::new(),
        workspaces: Vec::new(),
        truncated: PlatformSnapshotTruncation::default(),
    };
    assert!(PlatformSnapshot::new(fields.clone()).is_ok());
    fields.generated_at_ms = -1;
    assert!(PlatformSnapshot::new(fields.clone()).is_err());
    fields.generated_at_ms = 0;
    let hub = HubSummary {
        hub_id: HubId::from_uuid(Uuid::from_u128(1)).unwrap(),
        reachable: true,
        label: None,
    };
    fields.hubs = vec![hub; MAX_PLATFORM_SNAPSHOT_HUBS + 1];
    assert!(PlatformSnapshot::new(fields).is_err());
}
