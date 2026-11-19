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
