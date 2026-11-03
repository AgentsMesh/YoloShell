use crate::{State, events, network::Update};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};

#[test]
fn rejected_startup_negotiation_is_not_a_ready_connection() {
    let mut state = crate::client_capability_support::state();
    assert!(state.connection_ready());
    state.client_startup_failure = Some("startup rejected".into());
    assert!(!state.connection_ready());
}

#[test]
fn live_ready_projects_client_capabilities_without_ui_types() {
    let mut state = State::default();
    let (jobs, _) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(Uuid::new_v4())),
    );

    assert!(state.client_startup_failure.is_none());
    assert!(state.client_capability_enabled(ClientCapability::TerminalControl));
    assert!(state.client_capability_enabled(ClientCapability::AgentObservation));
}

#[test]
fn missing_startup_method_is_recorded_before_actions_are_replayed() {
    let mut state = State::default();
    let (jobs, mut received) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let hello = Hello {
        hub_id: yolo_hub_identity::HubId::new(),
        version: HUB_PROTOCOL_VERSION,
        methods: vec![Method::Hello],
    };
    events::apply(
        &mut state,
        &jobs,
        Update::Control(yolo_hub_client_channels::ControlEvent::Ready {
            generation: Uuid::new_v4(),
            hello,
        }),
    );

    assert!(state.client_capabilities.is_some());
    assert!(state.client_startup_failure.is_some());
    assert!(!state.client_capability_enabled(ClientCapability::TerminalControl));
    assert!(state.control_generation.is_none());
    assert!(received.try_recv().is_err());
}

#[test]
fn connection_loss_retires_its_capability_projection() {
    let mut state = State::default();
    let (jobs, _) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(crate::client_capability_support::ready(Uuid::new_v4())),
    );
    assert!(state.client_capability_enabled(ClientCapability::TerminalControl));
    state.disconnected();
    assert!(state.client_capabilities.is_none());
}
