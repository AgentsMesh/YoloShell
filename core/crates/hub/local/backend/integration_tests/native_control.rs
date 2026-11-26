use super::{
    cleanup_rpc::*,
    fixtures::agent_control::{authority, request},
    native_support::*,
};
use yolo_hub_agent_protocol::{Capability, CommandOutcome};
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::{
    protocol::{ResponseEnvelope, method},
    worker::{NativeControlOutcome, NativeControlResult, NativeInterruptRequest},
};

#[tokio::test]
async fn coherent_native_witness_is_required_before_forwarding_exact_writer_and_control_identity() {
    for outcome in [
        NativeControlOutcome::Delivered,
        NativeControlOutcome::NotDelivered,
        NativeControlOutcome::Unknown,
    ] {
        let fixture = ProtocolFixture::new().with_agents();
        let view = source();
        let observed = observation(&view, SOURCE_REVISION);
        let snapshot = observe(&fixture, &view, Some(&observed)).await;
        assert_eq!(
            snapshot.fields().capabilities.interrupt,
            Capability::Available {}
        );
        let request = request(&snapshot);
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
                    assert_eq!(incoming.method, method::INTERRUPT_NATIVE_AGENT);
                    let fields: NativeInterruptRequest = incoming.decode_payload().unwrap();
                    assert_eq!(fields.request_id, request.fields().request_id);
                    assert_eq!(fields.terminal_id, view.manifest.terminal_id);
                    assert_eq!(
                        fields.runtime_epoch,
                        view.runtime.as_ref().unwrap().runtime_epoch
                    );
                    assert_eq!(fields.generation, grant.fields().generation);
                    assert_eq!(
                        fields.writer_capability.as_bytes(),
                        grant.fields().capability.as_uuid().as_bytes()
                    );
                    assert_eq!(&fields.witness, observed.witness());
                    assert_eq!(
                        fields.command_digest,
                        crate::native_control_receipt::command_digest(&request).unwrap()
                    );
                    ResponseEnvelope::success(
                        incoming,
                        &NativeControlResult {
                            terminal_id: fields.terminal_id,
                            runtime_epoch: fields.runtime_epoch,
                            request_id: fields.request_id,
                            command_digest: fields.command_digest,
                            outcome,
                        },
                    )
                    .unwrap()
                })
                .await;
        };
        let (reply, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        receipt(reply.unwrap(), &request, public_outcome(outcome));
        fixture.assert_no_pending_connection();
    }
}

#[tokio::test]
async fn stored_receipt_resolves_before_stale_cursor_or_writer_checks_without_dispatching_again() {
    for outcome in [
        NativeControlOutcome::Delivered,
        NativeControlOutcome::NotDelivered,
        NativeControlOutcome::Unknown,
    ] {
        let fixture = ProtocolFixture::new().with_agents();
        let view = source();
        let observed = observation(&view, SOURCE_REVISION);
        let original = observe(&fixture, &view, Some(&observed)).await;
        let request = request(&original);
        fixture
            .backend
            .agents
            .as_ref()
            .unwrap()
            .lock()
            .await
            .mark_unreachable()
            .unwrap();
        let command = Command::AgentControl {
            request: request.clone(),
            grant: authority(),
        };
        let (reply, ()) = bounded(async {
            tokio::join!(
                fixture.backend.execute(&command),
                fixture.reply(|incoming| lookup(incoming, &request, Some(outcome)))
            )
        })
        .await;
        receipt(reply.unwrap(), &request, public_outcome(outcome));
        fixture.assert_no_pending_connection();
    }
}

#[tokio::test]
async fn lost_effect_response_is_unknown_and_retry_only_queries_the_worker_journal() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let observed = observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    let request = request(&snapshot);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let peer = async {
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        refresh(&fixture, &view, Some(&observed)).await;
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        let (stream, incoming) = fixture.accept_request().await;
        assert_eq!(incoming.method, method::INTERRUPT_NATIVE_AGENT);
        drop(stream);
        // The lost reply triggers one durable-receipt probe before Unknown.
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
    };
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    receipt(reply.unwrap(), &request, CommandOutcome::Unknown {});
    let retry = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&retry),
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
