use super::{
    cleanup_rpc::*,
    native_support::*,
};
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::Capability;
use yolo_runtime::agent::AgentState;

fn idle_observation(view: &yolo_runtime::model::TerminalView, revision: u64) -> NativeControlObservation {
    let unready = observation(view, revision);
    NativeControlObservation::new(unready.binding().clone(), unready.witness().clone(), true).unwrap()
}

fn idle_view() -> yolo_runtime::model::TerminalView {
    let mut view = source();
    view.agent.as_mut().unwrap().state = AgentState::WaitingForInput;
    view
}

/// An unchanged idle binding keeps its stored witness: the catalogue sweep
/// reads only the Host catalogue, never the Worker's PTY-derived state.
#[tokio::test]
async fn unchanged_idle_binding_reuses_its_stored_observation() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = idle_view();
    let observed = idle_observation(&view, SOURCE_REVISION);
    let snapshot = observe(&fixture, &view, Some(&observed)).await;
    assert_eq!(
        snapshot.fields().capabilities.submit_prompt,
        Capability::Available {}
    );
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            sweep_reused(&fixture, &view)
        )
    })
    .await;
    let agents = agents.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].fields().capabilities.submit_prompt,
        Capability::Available {}
    );
    fixture.assert_no_pending_connection();
}

/// A binding that changed since the last observation still reaches the Worker.
#[tokio::test]
async fn changed_idle_binding_observes_the_worker_again() {
    let fixture = ProtocolFixture::new().with_agents();
    let view = idle_view();
    let observed = idle_observation(&view, SOURCE_REVISION);
    observe(&fixture, &view, Some(&observed)).await;
    let mut changed = view.clone();
    let binding = changed.agent.as_mut().unwrap();
    binding.updated_at_ms += 1;
    let replacement = observation(&changed, NEXT_REVISION);
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            sweep(&fixture, &changed, Some(&replacement))
        )
    })
    .await;
    let agents = agents.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(
        agents[0].fields().observed_unix_ms,
        changed.agent.as_ref().unwrap().updated_at_ms as u64
    );
    fixture.assert_no_pending_connection();
}
