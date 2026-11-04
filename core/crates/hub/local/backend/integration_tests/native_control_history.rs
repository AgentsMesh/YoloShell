use super::{
    cleanup_rpc::*,
    fixtures::{
        agent_control::{authority, request},
        constants::{MESSAGE, STEP},
    },
    native_support::*,
};
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, CommandOutcome, FailureCode, InteractionReference,
    InteractionResponse,
};
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::{
    model::RecoveryState,
    protocol::{ResponseEnvelope, method},
    worker::{NativeControlLookupRequest, NativeControlOutcome, NativeControlResult},
};

#[tokio::test]
async fn resumed_logical_agent_receipt_query_uses_retired_runtime_before_current_preconditions() {
    let fixture = ProtocolFixture::new().with_agents();
    let mut view = source();
    let original = observe(&fixture, &view, Some(&observation(&view, SOURCE_REVISION))).await;
    let request = request(&original);
    let old_runtime = view.runtime.as_ref().unwrap().runtime_epoch;
    view.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
    view.runtime.as_mut().unwrap().worker_id = uuid::Uuid::new_v4();
    view.manifest.recovery_state = RecoveryState::Resumed;
    let binding = view.agent.as_mut().unwrap();
    binding.process_pid = binding
        .process_pid
        .map(|pid| pid + u32::try_from(STEP).unwrap());
    binding.process_start_time = binding.process_start_time.map(|start| start + STEP);
    binding.updated_at_ms += i64::try_from(STEP).unwrap();
    let current = observe(&fixture, &view, Some(&observation(&view, NEXT_REVISION))).await;
    assert_eq!(
        current.cursor().target().address,
        original.cursor().target().address
    );
    assert_ne!(
        current.cursor().target().execution_id,
        original.cursor().target().execution_id
    );
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            fixture.reply(|incoming| {
                let fields: NativeControlLookupRequest = incoming.decode_payload().unwrap();
                assert_eq!(fields.runtime_epoch, old_runtime);
                lookup(incoming, &request, Some(NativeControlOutcome::Delivered))
            })
        )
    })
    .await;
    receipt(reply.unwrap(), &request, CommandOutcome::Delivered {});
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn malformed_lookup_correlation_is_unknown_and_cannot_fall_through_to_effect() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let snapshot = observe(&fixture, &view, Some(&observation(&view, SOURCE_REVISION))).await;
    let request = request(&snapshot);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            fixture.reply(|incoming| {
                assert_eq!(incoming.method, method::GET_NATIVE_CONTROL_RECEIPT);
                let fields: NativeControlLookupRequest = incoming.decode_payload().unwrap();
                ResponseEnvelope::success(
                    incoming,
                    &Some(NativeControlResult {
                        terminal_id: fields.terminal_id,
                        runtime_epoch: fields.runtime_epoch,
                        request_id: uuid::Uuid::new_v4(),
                        command_digest: fields.command_digest,
                        outcome: NativeControlOutcome::Delivered,
                    }),
                )
                .unwrap()
            })
        )
    })
    .await;
    receipt(reply.unwrap(), &request, CommandOutcome::Unknown {});
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn native_interrupt_capability_never_implies_text_response_or_approval_control() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let observed = observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    let mut fields = request(&snapshot).fields().clone();
    fields.action = AgentAction::Respond {
        interaction: InteractionReference {
            target: fields.target,
            id: uuid::Uuid::new_v4(),
        },
        response: InteractionResponse::Text {
            text: MESSAGE.into(),
        },
    };
    let request = AgentControlRequest::new(fields).unwrap();
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(fixture.backend.execute(&command), async {
            fixture
                .reply(|incoming| lookup(incoming, &request, None))
                .await;
            refresh(&fixture, &view, Some(&observed)).await;
            fixture
                .reply(|incoming| lookup(incoming, &request, None))
                .await;
        })
    })
    .await;
    receipt(
        reply.unwrap(),
        &request,
        CommandOutcome::Failed {
            code: FailureCode::Unsupported,
        },
    );
    fixture.assert_no_pending_connection();
}
