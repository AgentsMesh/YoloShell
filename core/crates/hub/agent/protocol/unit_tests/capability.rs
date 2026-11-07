use yolo_hub_agent_protocol::*;

use crate::fixtures::round_trip;

#[test]
fn capabilities_distinguish_support_from_runtime_availability() {
    assert_eq!(Capability::Available {}.require_available(), Ok(()));
    assert_eq!(
        Capability::Unsupported {}.require_available(),
        Err(ProtocolError::UnsupportedControl)
    );
    for reason in [
        UnavailabilityReason::NotRunning,
        UnavailabilityReason::NoPendingInteraction,
        UnavailabilityReason::InputUnavailable,
        UnavailabilityReason::AdapterUnavailable,
        UnavailabilityReason::StateUnknown,
    ] {
        let unavailable = Capability::Unavailable { reason };
        assert_eq!(
            unavailable.require_available(),
            Err(ProtocolError::UnavailableControl)
        );
        assert_eq!(round_trip(&unavailable), unavailable);
    }
    assert_eq!(
        round_trip(&AgentCapabilities::UNSUPPORTED),
        AgentCapabilities::UNSUPPORTED
    );
}

#[test]
fn state_dimensions_round_trip_independently() {
    for value in [
        Lifecycle::Unknown {},
        Lifecycle::Starting {},
        Lifecycle::Running {},
        Lifecycle::Exited { exit_code: None },
        Lifecycle::Exited {
            exit_code: Some(i32::MIN),
        },
    ] {
        assert_eq!(round_trip(&value), value);
    }
    for value in [
        Activity::Unknown,
        Activity::Idle,
        Activity::Working,
        Activity::WaitingForInput,
        Activity::WaitingForApproval,
    ] {
        assert_eq!(round_trip(&value), value);
    }
    for value in [Reachability::Reachable, Reachability::Unreachable] {
        assert_eq!(round_trip(&value), value);
    }
    for value in [Freshness::Current, Freshness::Stale] {
        assert_eq!(round_trip(&value), value);
    }
    for value in [
        ObservationSource::ProviderHook,
        ObservationSource::ProcessObserver,
        ObservationSource::Connector,
    ] {
        assert_eq!(round_trip(&value), value);
    }
    for value in [
        RecoveryStatus::Unknown,
        RecoveryStatus::Unsupported,
        RecoveryStatus::Unavailable,
        RecoveryStatus::Available,
    ] {
        assert_eq!(round_trip(&value), value);
    }
}
