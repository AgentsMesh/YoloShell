use super::{cleanup_rpc::*, native_support::*};
use yolo_hub_rpc_protocol::Reply;
use yolo_runtime::protocol::{ResponseEnvelope, method};
use yolo_runtime_protocol::TerminalRef;

/// A get/control reads only its target session; the catalogue and other
/// sessions' Workers are never touched.
#[tokio::test]
async fn scoped_get_reads_only_the_requested_session() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let observed = observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    let target = snapshot.cursor().target();
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.get_agent(target),
            async {
                fixture
                    .reply(|request| {
                        assert_eq!(request.method, method::GET_SESSION);
                        let fields: TerminalRef = request.decode_payload().unwrap();
                        assert_eq!(fields.terminal_id, view.manifest.terminal_id);
                        ResponseEnvelope::success(request, &view).unwrap()
                    })
                    .await;
                fixture
                    .reply(|request| native(request, &view, Some(&observed)))
                    .await;
            }
        )
    })
    .await;
    let Reply::Agent { agent } = reply.unwrap() else {
        panic!("expected an agent reply")
    };
    assert_eq!(agent.cursor().target(), target);
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn current_agent_and_platform_snapshot_each_use_one_live_sweep() {
    use yolo_hub_rpc_protocol::{
        GetPlatformSnapshotFields, GetPlatformSnapshotRequest, PLATFORM_SNAPSHOT_SCHEMA_VERSION,
    };
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let observed = observation(&view, SOURCE_REVISION);
    let target = observe(&fixture, &view, Some(&observed))
        .await
        .cursor()
        .target();
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.get_current_agent(target.address),
            sweep(&fixture, &view, Some(&observed))
        )
    })
    .await;
    let Reply::Agent { agent } = reply.unwrap() else {
        panic!("expected a current agent")
    };
    assert_eq!(agent.cursor().target(), target);
    const SECTION_LIMIT: u16 = 1;
    let request = GetPlatformSnapshotRequest::new(GetPlatformSnapshotFields {
        schema_version: PLATFORM_SNAPSHOT_SCHEMA_VERSION,
        agent_limit: SECTION_LIMIT,
        terminal_limit: SECTION_LIMIT,
        run_limit: SECTION_LIMIT,
        workspace_limit: SECTION_LIMIT,
    })
    .unwrap();
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.platform_snapshot(&request),
            sweep(&fixture, &view, Some(&observed))
        )
    })
    .await;
    let Reply::PlatformSnapshot { snapshot } = reply.unwrap() else {
        panic!("expected a platform snapshot")
    };
    assert_eq!(snapshot.fields().agents[0].cursor().target(), target);
    assert_eq!(snapshot.fields().terminals[0].fields().agent, Some(target));
    fixture.assert_no_pending_connection();
}
