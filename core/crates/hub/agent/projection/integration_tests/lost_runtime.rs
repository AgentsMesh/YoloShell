use super::{
    constants::IDENTITY_STEP,
    control_support::{observation, request},
    fixtures::{directory, open},
    view::view,
};
use yolo_hub_agent_protocol::{
    Activity, Capability, Freshness, Lifecycle, Reachability, UnavailabilityReason,
};
use yolo_terminal_model::TerminalRuntimeStatus;

#[test]
fn lost_terminal_preserves_agent_identity_without_claiming_liveness_exit_or_control() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let mut source = view();
    let witness = observation(&source, IDENTITY_STEP);
    let first = projection
        .project_with_observation(&source, Some(&witness))
        .unwrap()
        .unwrap();
    assert_eq!(
        first.fields().capabilities.interrupt,
        Capability::Available {}
    );

    source.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Lost;
    let lost = projection
        .project_with_observation(&source, Some(&witness))
        .unwrap()
        .unwrap();
    assert_eq!(lost.cursor().target(), first.cursor().target());
    assert_eq!(lost.fields().reachability, Reachability::Unreachable);
    assert_eq!(lost.fields().freshness, Freshness::Stale);
    assert_eq!(lost.fields().lifecycle, Lifecycle::Unknown {});
    assert_eq!(lost.fields().activity, Activity::Unknown);
    assert_eq!(
        lost.fields().capabilities.interrupt,
        Capability::Unavailable {
            reason: UnavailabilityReason::StateUnknown,
        }
    );
    assert!(projection.control_witness(&request(&lost)).is_err());
    assert_eq!(projection.list().unwrap(), vec![lost.clone()]);
    assert_eq!(projection.list_active().unwrap(), vec![lost.clone()]);
    assert_eq!(
        projection
            .project_with_observation(&source, Some(&witness))
            .unwrap(),
        Some(lost)
    );
}
