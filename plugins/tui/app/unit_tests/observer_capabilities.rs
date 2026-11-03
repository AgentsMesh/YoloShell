use super::{creation_support::Fixture, finalization_support, fixtures};
use crate::{
    Mode, StartupAction, State, events,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_catalog_client::Catalog;
use yolo_hub_client_channels::{CatalogEvent, ControlEvent};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello};
use yolo_hub_terminal_protocol::TerminalAction;

fn observer_ready(generation: Uuid) -> ControlEvent {
    ControlEvent::Ready {
        generation,
        hello: Hello {
            hub_id: yolo_hub_identity::HubId::new(),
            version: HUB_PROTOCOL_VERSION,
            methods: yolo_builtin_catalog::tui_api().required_methods,
        },
    }
}

#[test]
fn observer_ready_catalog_and_attach_never_request_control_or_write() {
    let shell = fixtures::session();
    let address = shell.fields().address;
    let mut state = State::default();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(observer_ready(Uuid::new_v4())),
    );
    assert!(state.client_startup_failure.is_none());
    let Job::Refresh { request_id } = incoming.try_recv().unwrap() else {
        panic!()
    };
    events::apply(
        &mut state,
        &jobs,
        Update::Catalog(CatalogEvent {
            request_id,
            result: Ok(Catalog {
                terminals: vec![shell],
                ..Catalog::default()
            }),
        }),
    );
    crate::startup::install(
        &mut state,
        address.hub_id,
        StartupAction::Attach {
            selector: Some(address.to_string()),
        },
    );
    crate::startup::apply(&mut state, &jobs);
    assert_eq!(state.focused_terminal(), Some(address));
    assert_eq!(state.mode, Mode::Terminal);
    assert!(state.initial_control.is_none());
    crate::polling::renew_leases(&mut state, &jobs);
    crate::control::refresh_observations(&mut state, &jobs);
    crate::control::reconcile(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        crate::control::detail(&state, address),
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE
    );
    assert!(!crate::control::detail(&state, address).contains("take control"));
}

#[test]
fn observer_rejects_explicit_mutations_without_changing_the_tab_or_preparing_a_request() {
    let mut state = State::default();
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    events::apply(
        &mut state,
        &jobs,
        Update::Control(observer_ready(Uuid::new_v4())),
    );
    incoming.try_recv().unwrap();
    let shell = fixtures::session();
    let target = shell.fields().runtime.unwrap().target;
    state.replace_terminals(vec![shell.clone()]);
    state.bind_focused(Some(target.address)).unwrap();
    state.selected = Some(target.address);
    state.mode = Mode::Terminal;
    let tabs = state.tab_snapshot();
    assert!(!crate::shell_actions::create(
        &mut state,
        &jobs,
        target.address.hub_id,
        fixtures::SESSION_DIRECTORY.into(),
        None
    ));
    assert_eq!(
        state.status,
        crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE
    );
    crate::form_actions::lifecycle(
        &mut state,
        &jobs,
        TerminalAction::Terminate { force: false },
    );
    assert!(state.confirmation.is_none());
    assert_eq!(state.tab_snapshot(), tabs);
    assert_eq!(state.creations.len(), 0);
    crate::polling::resume_input(&mut state, &jobs, true);
    assert_eq!(
        state.status,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE
    );
    assert!(!crate::terminal_actions::deliver(
        &mut state,
        &jobs,
        Ok(b"input".to_vec())
    ));
    state.record_grant(fixtures::grant(&shell), std::time::Instant::now());
    assert!(state.writers.is_empty());
    assert!(incoming.try_recv().is_err());
}

#[test]
fn observer_connection_keeps_recovery_evidence_without_query_or_finalize_retries() {
    let mut pending = Fixture::new();
    let request = pending.start();
    let yolo_hub_rpc_protocol::Command::Create { request } = request.command else {
        panic!()
    };
    pending
        .state
        .creations
        .retire_at(&request, std::time::Instant::now());
    let mut confirmed = Fixture::new();
    finalization_support::confirmed(&mut confirmed);
    for fixture in [&mut pending, &mut confirmed] {
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Control(observer_ready(Uuid::new_v4())),
        );
        assert!(matches!(
            fixture.incoming.try_recv().unwrap(),
            Job::Refresh { .. }
        ));
        let retained = fixture.state.creations.len();
        assert!(retained > 0);
        crate::polling::renew_leases(&mut fixture.state, &fixture.jobs);
        crate::creation_recovery::flush(
            &mut fixture.state,
            &fixture.jobs,
            finalization_support::later(),
        );
        crate::finalization_actions::flush(
            &mut fixture.state,
            &fixture.jobs,
            finalization_support::later(),
        );
        assert_eq!(fixture.state.creations.len(), retained);
        assert!(fixture.incoming.try_recv().is_err());
    }
}
