use super::{
    cleanup_rpc::*,
    fixtures::agent_control::{authority, request},
    native_support::*,
};
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, Capability, CommandOutcome, Prompt,
};
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::{
    agent::AgentState,
    protocol::{ResponseEnvelope, method},
    worker::{NativeControlOutcome, NativeControlResult, NativeSubmitRequest},
};

const PROMPT: &str = "Continue the existing native conversation\n你好";

#[tokio::test]
async fn public_prompt_routes_with_writer_witness_and_recovers_lost_reply_without_replay() {
    let fixture = ProtocolFixture::new().with_agents();
    let mut view = source();
    view.agent.as_mut().unwrap().state = AgentState::WaitingForInput;
    let unready = observation(&view, SOURCE_REVISION);
    let observed =
        NativeControlObservation::new(unready.binding().clone(), unready.witness().clone(), true)
            .unwrap();
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    assert_eq!(
        snapshot.fields().capabilities.submit_prompt,
        Capability::Available {}
    );
    let mut fields = request(&snapshot).fields().clone();
    fields.action = AgentAction::SubmitPrompt {
        prompt: Prompt::new(PROMPT.into()).unwrap(),
    };
    let request = AgentControlRequest::new(fields).unwrap();
    let grant = authority();
    let command = Command::AgentControl {
        request: request.clone(),
        grant,
    };
    let peer = async {
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        refresh_reused(&fixture, &view).await;
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        let (stream, incoming) = fixture.accept_request().await;
        assert_eq!(incoming.method, method::SUBMIT_NATIVE_AGENT);
        let sent: NativeSubmitRequest = incoming.decode_payload().unwrap();
        assert_eq!(sent.prompt.text(), PROMPT);
        assert_eq!(sent.witness, *observed.witness());
        assert_eq!(sent.request_id, request.fields().request_id);
        assert_eq!(sent.terminal_id, view.manifest.terminal_id);
        assert_eq!(
            sent.runtime_epoch,
            view.runtime.as_ref().unwrap().runtime_epoch
        );
        assert_eq!(sent.generation, grant.fields().generation);
        assert_eq!(
            sent.writer_capability.as_bytes(),
            grant.fields().capability.as_uuid().as_bytes()
        );
        assert_eq!(
            sent.command_digest,
            crate::native_control_receipt::command_digest(&request).unwrap()
        );
        drop(stream);
        // The lost reply triggers one durable-receipt probe before Unknown.
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
    };
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    receipt(reply.unwrap(), &request, CommandOutcome::Unknown {});
    // Even a stale public cursor may query the durable receipt, but never dispatch again.
    fixture
        .backend
        .agents
        .as_ref()
        .unwrap()
        .lock()
        .await
        .mark_unreachable()
        .unwrap();
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            fixture.reply(|incoming| lookup(
                incoming,
                &request,
                Some(NativeControlOutcome::Delivered)
            ))
        )
    })
    .await;
    receipt(reply.unwrap(), &request, CommandOutcome::Delivered {});
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn immediate_submit_delivery_remains_delivered_and_never_fabricates_agent_confirmation() {
    let fixture = ProtocolFixture::new().with_agents();
    let mut view = source();
    view.agent.as_mut().unwrap().state = AgentState::WaitingForInput;
    let unready = observation(&view, SOURCE_REVISION);
    let observed =
        NativeControlObservation::new(unready.binding().clone(), unready.witness().clone(), true)
            .unwrap();
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    let mut fields = request(&snapshot).fields().clone();
    fields.action = AgentAction::SubmitPrompt {
        prompt: Prompt::new(PROMPT.into()).unwrap(),
    };
    let request = AgentControlRequest::new(fields).unwrap();
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let peer = async {
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        refresh_reused(&fixture, &view).await;
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        fixture
            .reply(|incoming| {
                assert_eq!(incoming.method, method::SUBMIT_NATIVE_AGENT);
                let sent: NativeSubmitRequest = incoming.decode_payload().unwrap();
                ResponseEnvelope::success(
                    incoming,
                    &NativeControlResult {
                        terminal_id: sent.terminal_id,
                        runtime_epoch: sent.runtime_epoch,
                        request_id: sent.request_id,
                        command_digest: sent.command_digest,
                        outcome: NativeControlOutcome::Delivered,
                    },
                )
                .unwrap()
            })
            .await;
    };
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    receipt(reply.unwrap(), &request, CommandOutcome::Delivered {});
    fixture.assert_no_pending_connection();
}
