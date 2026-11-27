use super::{
    constants::*,
    control_support::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_agent_contract::AgentState;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::{
    Activity, Capability, Freshness, Lifecycle, ObservationSource, RecoveryStatus,
    UnavailabilityReason,
};

#[test]
fn unresolved_activity_keeps_execution_and_recovery_but_cannot_advertise_prompt_input() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    let mut previous = None;
    for (state, activity) in [
        (AgentState::WaitingForInput, Activity::WaitingForInput),
        (AgentState::Active, Activity::Unknown),
        (AgentState::Running, Activity::Working),
        (AgentState::Active, Activity::Unknown),
    ] {
        let binding = source.agent.as_mut().unwrap();
        binding.state = state;
        binding.updated_at_ms += IDENTITY_STEP as i64;
        let revision = binding.updated_at_ms as u64;
        let observed = observation(&source, revision);
        let ready = state == AgentState::WaitingForInput;
        let observed = NativeControlObservation::new(
            observed.binding().clone(),
            observed.witness().clone(),
            ready,
        )
        .unwrap();
        let snapshot = store
            .project_with_observation(&source, Some(&observed))
            .unwrap()
            .unwrap();
        let fields = snapshot.fields();
        assert_eq!(fields.lifecycle, Lifecycle::Running {});
        assert_eq!(fields.activity, activity);
        assert_eq!(fields.freshness, Freshness::Current);
        assert_eq!(fields.source, ObservationSource::ProviderHook);
        assert_eq!(fields.recovery, RecoveryStatus::Available);
        assert_eq!(fields.capabilities.interrupt, Capability::Available {});
        assert_eq!(
            fields.capabilities.submit_prompt,
            if ready {
                Capability::Available {}
            } else {
                Capability::Unavailable {
                    reason: UnavailabilityReason::InputUnavailable,
                }
            }
        );
        if let Some(target) = previous {
            assert_eq!(snapshot.cursor().target(), target);
        }
        previous = Some(snapshot.cursor().target());
        assert_eq!(store.list_active().unwrap(), vec![snapshot.clone()]);
        assert_eq!(
            store
                .project_with_observation(&source, Some(&observed))
                .unwrap(),
            Some(snapshot)
        );
    }
}
