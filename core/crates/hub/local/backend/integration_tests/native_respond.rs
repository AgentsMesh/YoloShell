use super::{
    cleanup_rpc::*,
    fixtures::agent_control::{authority, request},
    native_support::*,
};
use uuid::Uuid;
use yolo_agent_control_contract::{
    NativeControlObservation, NativeInteraction, NativeInteractionChoice, NativeInteractionKind,
};
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, Capability, CommandOutcome, InteractionResponse,
};
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::{
    protocol::{ResponseEnvelope, method},
    worker::{NativeControlOutcome, NativeControlResult, NativeRespondRequest},
};

#[tokio::test]
async fn public_interaction_response_routes_the_exact_choice_and_witness() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let base = observation(&view, SOURCE_REVISION);
    let pending = NativeInteraction::new(
        Uuid::new_v4(),
        NativeInteractionKind::Approval,
        vec![
            NativeInteractionChoice {
                id: "allow".into(),
                label: "Allow".into(),
            },
            NativeInteractionChoice {
                id: "deny".into(),
                label: "Deny".into(),
            },
        ],
        "Bash: cargo test".into(),
    )
    .unwrap();
    let observed = NativeControlObservation::new_with_interaction(
        base.binding().clone(),
        base.witness().clone(),
        false,
        None,
        Some(pending.clone()),
    )
    .unwrap();
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    assert_eq!(
        snapshot.fields().capabilities.respond_interaction,
        Capability::Available {}
    );
    let mut fields = request(&snapshot).fields().clone();
    fields.action = AgentAction::Respond {
        interaction: snapshot.fields().interaction.as_ref().unwrap().reference(),
        response: InteractionResponse::Choice {
            choice: "allow".into(),
        },
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
        refresh(&fixture, &view, Some(&observed)).await;
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        fixture
            .reply(|incoming| {
                assert_eq!(incoming.method, method::RESPOND_NATIVE_AGENT);
                let sent: NativeRespondRequest = incoming.decode_payload().unwrap();
                assert_eq!(sent.interaction, pending.id());
                assert_eq!(sent.choice, "allow");
                assert_eq!(&sent.witness, observed.witness());
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
