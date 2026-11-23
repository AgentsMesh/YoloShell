use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    fixtures::agent_control::host_view,
};
use yolo_agent_control_contract::*;
use yolo_hub_agent_protocol::{AgentControlRequest, AgentSnapshot, CommandOutcome, FailureCode};
use yolo_hub_rpc_protocol::Reply;
use yolo_runtime::{
    host::{ListSessionsRequest, ListSessionsResult},
    model::TerminalView,
    protocol::{RequestEnvelope, ResponseEnvelope, capability, method},
    worker::*,
};

pub const BOOT: &str = "backend-native-control-boot";
pub const SOURCE_REVISION: u64 = 1;
pub const NEXT_REVISION: u64 = 2;

pub fn source() -> TerminalView {
    let mut view = host_view();
    view.runtime.as_mut().unwrap().capabilities =
        Some(vec![capability::NATIVE_AGENT_CONTROL.into()]);
    view
}
pub fn observation(view: &TerminalView, revision: u64) -> NativeControlObservation {
    let binding = view.agent.as_ref().unwrap().clone();
    let runtime = view.runtime.as_ref().unwrap();
    let witness = NativeControlWitness::new(NativeControlWitnessFields {
        terminal_id: view.manifest.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        boot_id: BOOT.into(),
        kind: binding.kind.clone(),
        process_pid: binding.process_pid.unwrap(),
        process_start_time: binding.process_start_time.unwrap(),
        binding_digest: binding_digest(&binding).unwrap(),
        authority_digest: authority_digest(&revision).unwrap(),
    })
    .unwrap();
    NativeControlObservation::new(binding, witness, false).unwrap()
}
pub fn catalogue(request: &RequestEnvelope, view: &TerminalView) -> ResponseEnvelope {
    live_listing(request, std::slice::from_ref(view))
}
/// The complete catalogue, including archived sessions.
pub fn archived_listing(request: &RequestEnvelope, views: &[TerminalView]) -> ResponseEnvelope {
    listing(request, true, views)
}
/// The live catalogue; the agent sweep reads only this scope.
pub fn live_listing(request: &RequestEnvelope, views: &[TerminalView]) -> ResponseEnvelope {
    listing(request, false, views)
}
fn listing(
    request: &RequestEnvelope,
    include_archived: bool,
    views: &[TerminalView],
) -> ResponseEnvelope {
    assert_eq!(request.method, method::LIST_SESSIONS);
    assert_eq!(
        request
            .decode_payload::<ListSessionsRequest>()
            .unwrap()
            .include_archived,
        include_archived
    );
    ResponseEnvelope::success(
        request,
        &ListSessionsResult {
            sessions: views.to_vec(),
            complete: true,
        },
    )
    .unwrap()
}
pub fn session(request: &RequestEnvelope, view: &TerminalView) -> ResponseEnvelope {
    assert_eq!(request.method, method::GET_SESSION);
    let fields: yolo_runtime_protocol::TerminalRef = request.decode_payload().unwrap();
    assert_eq!(fields.terminal_id, view.manifest.terminal_id);
    ResponseEnvelope::success(request, view).unwrap()
}
pub fn native(
    request: &RequestEnvelope,
    view: &TerminalView,
    observation: Option<&NativeControlObservation>,
) -> ResponseEnvelope {
    assert_eq!(request.method, method::OBSERVE_NATIVE_AGENT);
    let fields: NativeObservationRequest = request.decode_payload().unwrap();
    assert_eq!(fields.terminal_id, view.manifest.terminal_id);
    assert_eq!(
        fields.runtime_epoch,
        view.runtime.as_ref().unwrap().runtime_epoch
    );
    assert_eq!(
        fields.binding_digest,
        binding_digest(view.agent.as_ref().unwrap()).unwrap()
    );
    ResponseEnvelope::success(request, &observation).unwrap()
}
/// A scoped get/control reads its target session, then observes it.
pub async fn refresh(
    fixture: &ProtocolFixture,
    view: &TerminalView,
    observation: Option<&NativeControlObservation>,
) {
    fixture.reply(|request| session(request, view)).await;
    fixture
        .reply(|request| native(request, view, observation))
        .await;
}
/// An unchanged idle target reuses its stored observation: only the session
/// view is read again.
pub async fn refresh_reused(fixture: &ProtocolFixture, view: &TerminalView) {
    fixture.reply(|request| session(request, view)).await;
}
/// A catalogue sweep reads the live catalogue, then observes one binding.
pub async fn sweep(
    fixture: &ProtocolFixture,
    view: &TerminalView,
    observation: Option<&NativeControlObservation>,
) {
    fixture.reply(|request| catalogue(request, view)).await;
    fixture
        .reply(|request| native(request, view, observation))
        .await;
}
/// An unchanged idle binding in a catalogue sweep needs no Worker call.
pub async fn sweep_reused(fixture: &ProtocolFixture, view: &TerminalView) {
    fixture.reply(|request| catalogue(request, view)).await;
}
pub async fn observe(
    fixture: &ProtocolFixture,
    view: &TerminalView,
    observation: Option<&NativeControlObservation>,
) -> AgentSnapshot {
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            sweep(fixture, view, observation)
        )
    })
    .await;
    let mut agents = agents.unwrap();
    assert_eq!(agents.len(), super::fixtures::agent_control::SINGLE_AGENT);
    agents.pop().unwrap()
}
pub fn lookup(
    request: &RequestEnvelope,
    command: &AgentControlRequest,
    result: Option<NativeControlOutcome>,
) -> ResponseEnvelope {
    assert_eq!(request.method, method::GET_NATIVE_CONTROL_RECEIPT);
    let fields: NativeControlLookupRequest = request.decode_payload().unwrap();
    assert_eq!(fields.request_id, command.fields().request_id);
    assert_eq!(
        fields.command_digest,
        crate::native_control_receipt::command_digest(command).unwrap()
    );
    let result = result.map(|outcome| NativeControlResult {
        terminal_id: fields.terminal_id,
        runtime_epoch: fields.runtime_epoch,
        request_id: fields.request_id,
        command_digest: fields.command_digest,
        outcome,
    });
    ResponseEnvelope::success(request, &result).unwrap()
}
pub fn receipt(reply: Reply, request: &AgentControlRequest, expected: CommandOutcome) {
    let Reply::AgentControl { receipt } = reply else {
        panic!()
    };
    assert_eq!(receipt.request_id(), request.fields().request_id);
    assert_eq!(receipt.target(), request.fields().target);
    assert_eq!(receipt.outcome(), expected);
}
pub fn public_outcome(outcome: NativeControlOutcome) -> CommandOutcome {
    match outcome {
        NativeControlOutcome::Accepted => CommandOutcome::Accepted {},
        NativeControlOutcome::Confirmed => CommandOutcome::Confirmed {},
        NativeControlOutcome::Delivered => CommandOutcome::Delivered {},
        NativeControlOutcome::NotDelivered => CommandOutcome::Failed {
            code: FailureCode::AdapterRejected,
        },
        NativeControlOutcome::Unknown => CommandOutcome::Unknown {},
    }
}
