use super::{
    cleanup_rpc::*,
    fixtures::agent_control::{authority, request},
    native_support::*,
};
use yolo_hub_agent_protocol::CommandOutcome;
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::worker::NativeControlOutcome;

#[tokio::test]
async fn pending_prior_effect_receipt_wins_over_source_change_observed_while_retry_waited() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let original = observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&original)).await;
    let request = request(&snapshot);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let changed = observation(&view, NEXT_REVISION);
    let peer = async {
        // The earlier delivered call was still pending when this first lookup ran.
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        refresh(&fixture, &view, Some(&changed)).await;
        // Source refresh crosses the Worker's admission gate after that call finishes.
        fixture
            .reply(|incoming| lookup(incoming, &request, Some(NativeControlOutcome::Delivered)))
            .await;
    };
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    receipt(reply.unwrap(), &request, CommandOutcome::Delivered {});
    let projection = fixture.backend.agents.as_ref().unwrap().lock().await;
    let current = projection
        .get(snapshot.cursor().target().address)
        .unwrap()
        .unwrap();
    assert!(current.cursor().sequence() > snapshot.cursor().sequence());
    fixture.assert_no_pending_connection();
}
