use super::{
    cleanup_rpc::*,
    fixtures::{
        agent_control::{authority, request},
        constants::MESSAGE,
    },
    native_support::*,
};
use yolo_agent_control_contract::{NativeControlObservation, NativeControlWitness};
use yolo_hub_agent_protocol::{Capability, CommandOutcome, FailureCode};
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::protocol::{ErrorBody, ResponseEnvelope, error_code, method};

#[tokio::test]
async fn changed_source_witness_advances_cursor_and_rejects_old_control_without_effect() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let before = observation(&view, SOURCE_REVISION);
    let original = observe(&fixture, &view, Some(&before)).await;
    let request = request(&original);
    let replacement = observation(&view, NEXT_REVISION);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: authority(),
    };
    let peer = async {
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
        refresh(&fixture, &view, Some(&replacement)).await;
        fixture
            .reply(|incoming| lookup(incoming, &request, None))
            .await;
    };
    let (reply, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    receipt(
        reply.unwrap(),
        &request,
        CommandOutcome::Failed {
            code: FailureCode::PreconditionsChanged,
        },
    );
    let projection = fixture.backend.agents.as_ref().unwrap().lock().await;
    let current = projection
        .get(original.cursor().target().address)
        .unwrap()
        .unwrap();
    assert!(current.cursor().sequence() > original.cursor().sequence());
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn unverifiable_receipt_runtime_returns_unknown_without_source_read_or_second_effect() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = source();
    let observed = observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
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
                ResponseEnvelope::failure(
                    incoming,
                    ErrorBody::new(error_code::WORKER_UNREACHABLE, MESSAGE, false),
                )
            })
        )
    })
    .await;
    receipt(reply.unwrap(), &request, CommandOutcome::Unknown {});
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn advertised_method_without_coherent_worker_witness_never_enables_control() {
    for foreign_worker in [false, true] {
        let fixture = ProtocolFixture::new().with_agents();
        let view = source();
        let observed = observation(&view, SOURCE_REVISION);
        let mut fields = observed.witness().fields().clone();
        fields.worker_id = uuid::Uuid::new_v4();
        let foreign = NativeControlObservation::new(
            observed.binding().clone(),
            NativeControlWitness::new(fields).unwrap(),
            false,
        )
        .unwrap();
        let observation = foreign_worker.then_some(&foreign);
        let snapshot = observe(&fixture, &view, observation).await;
        assert_eq!(
            snapshot.fields().capabilities.interrupt,
            Capability::Unsupported {}
        );
        let request = request(&snapshot);
        let command = Command::AgentControl {
            request: request.clone(),
            grant: authority(),
        };
        let (reply, ()) = bounded(async {
            tokio::join!(
                fixture.backend.execute(&command),
                refresh(&fixture, &view, observation)
            )
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
}
