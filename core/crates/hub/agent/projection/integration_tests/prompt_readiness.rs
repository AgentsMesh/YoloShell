use super::{
    constants::*,
    control_support::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_agent_contract::AgentState;
use yolo_agent_control_contract::{NativeControlObservation, NativeTurnEnd, NativeTurnEndKind};
use yolo_hub_agent_protocol::{Activity, Capability, Lifecycle, UnavailabilityReason};

#[test]
fn prompt_capability_tracks_worker_readiness_and_invalidates_old_control_cursor() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let mut view = view();
    view.agent.as_mut().unwrap().state = AgentState::WaitingForInput;
    let unavailable = observation(&view, IDENTITY_STEP);
    let before = projection
        .project_with_observation(&view, Some(&unavailable))
        .unwrap()
        .unwrap();
    assert_eq!(
        before.fields().capabilities.submit_prompt,
        Capability::Unavailable {
            reason: UnavailabilityReason::InputUnavailable,
        }
    );
    let ready = NativeControlObservation::new(
        unavailable.binding().clone(),
        unavailable.witness().clone(),
        true,
    )
    .unwrap();
    let current = projection
        .project_with_observation(&view, Some(&ready))
        .unwrap()
        .unwrap();
    assert_eq!(current.cursor().target(), before.cursor().target());
    assert!(current.cursor().sequence() > before.cursor().sequence());
    assert_eq!(
        current.fields().capabilities.submit_prompt,
        Capability::Available {}
    );
    assert!(projection.control_witness(&request(&before)).is_err());
    let lost = projection.project(&view).unwrap().unwrap();
    assert_eq!(
        lost.fields().capabilities.submit_prompt,
        Capability::Unavailable {
            reason: UnavailabilityReason::StateUnknown,
        }
    );
    assert!(projection.control_witness(&request(&current)).is_err());
}

#[test]
fn provider_turn_end_projects_waiting_for_input_on_a_stop_candidate() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let mut view = view();
    view.agent.as_mut().unwrap().state = AgentState::Active;
    let base = observation(&view, IDENTITY_STEP);
    let ready = NativeControlObservation::new_with_turn_end(
        base.binding().clone(),
        base.witness().clone(),
        true,
        Some(NativeTurnEnd {
            turn_id: "turn-1".into(),
            kind: NativeTurnEndKind::Completed,
        }),
    )
    .unwrap();
    let snapshot = projection
        .project_with_observation(&view, Some(&ready))
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.fields().lifecycle, Lifecycle::Running {});
    assert_eq!(snapshot.fields().activity, Activity::WaitingForInput);
    assert_eq!(
        snapshot.fields().capabilities.submit_prompt,
        Capability::Available {}
    );
}
