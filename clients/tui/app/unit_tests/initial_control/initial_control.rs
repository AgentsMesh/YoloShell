use super::{creation_support::*, fixtures};
use crate::{
    events,
    network::{Job, Update},
};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::TerminalSummary;

const EXPIRED_CREATION_OFFSET: std::time::Duration = std::time::Duration::from_secs(1);

fn created_session(address: TerminalAddress) -> TerminalSummary {
    let mut fields = fixtures::session().fields().clone();
    fields.address = address;
    if let Some(runtime) = fields.runtime.as_mut() {
        runtime.target.address = address;
    }
    TerminalSummary::new(fields).unwrap()
}

#[test]
fn live_created_binding_enqueues_one_fresh_acquire_after_authoritative_catalog() {
    let mut fixture = Fixture::new();
    let create = fixture.start();
    let result = fixture.created();
    fixture.finish(create.clone(), result);
    fixture.assert_finalize(&create, result, true);
    fixture
        .state
        .replace_terminals(vec![created_session(result.address)]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(matches!(
        fixture.incoming.try_recv(),
        Ok(Job::InitialAcquire { request, .. }) if request.target.address == result.address
    ));
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(fixture.incoming.try_recv().is_err());
    assert_eq!(fixture.state.mode, crate::Mode::Terminal);
}

#[test]
fn confirmed_creation_uses_a_new_control_budget_after_create_deadline() {
    let mut fixture = Fixture::new();
    let create = fixture.start();
    let request_id = match &create.command {
        Command::Create { request } => request.fields().request_id,
        _ => panic!(),
    };
    fixture
        .state
        .creations
        .pending
        .get_mut(&request_id)
        .unwrap()
        .deadline = std::time::Instant::now() - EXPIRED_CREATION_OFFSET;
    let result = fixture.created();
    let confirmed_at = std::time::Instant::now();
    fixture.finish(create.clone(), result);
    fixture.assert_finalize(&create, result, true);
    let intent = fixture.state.initial_control.as_ref().unwrap();
    assert!(intent.deadline >= confirmed_at + crate::constants::INITIAL_CONTROL_TIMEOUT);
    assert!(intent.validity.is_current());
    fixture
        .state
        .replace_terminals(vec![created_session(result.address)]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(matches!(
        fixture.incoming.try_recv(),
        Ok(Job::InitialAcquire { .. })
    ));
}

#[test]
fn direct_creation_never_emits_takeover_and_a_competitor_keeps_the_terminal_read_only() {
    let mut fixture = Fixture::new();
    let create = fixture.start();
    let result = fixture.created();
    fixture.finish(create, result);
    fixture
        .state
        .replace_terminals(vec![created_session(result.address)]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    let mut acquire = None;
    while let Ok(job) = fixture.incoming.try_recv() {
        if let Job::InitialAcquire { request, .. } = job {
            acquire = Some(request);
        }
    }
    let request = acquire.unwrap();
    assert!(matches!(
        request.mode,
        yolo_hub_terminal_protocol::LeaseMode::Acquire {}
    ));
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Failed {
            command: Some(Command::Lease { request }),
            message: crate::constants::READ_ONLY_LABEL.into(),
            connected: true,
            transient: false,
        },
    );
    assert!(fixture.state.writers.is_empty());
    assert!(fixture.state.initial_control.is_none());
    assert_eq!(fixture.state.status, crate::constants::READ_ONLY_LABEL);
}

#[test]
fn user_navigation_cancels_initial_acquire_and_disconnect_drops_pending_intent() {
    let mut fixture = Fixture::new();
    let create = fixture.start();
    let result = fixture.created();
    fixture.finish(create, result);
    fixture
        .state
        .replace_terminals(vec![created_session(result.address)]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(fixture.state.initial_control.is_some());
    crate::initial_control::user_event(&mut fixture.state);
    assert!(
        fixture
            .state
            .initial_control
            .as_ref()
            .is_some_and(|intent| !intent.validity.is_current())
    );
    crate::initial_control::disconnected(&mut fixture.state);
    assert!(fixture.state.initial_control.is_none());
}

#[test]
fn delayed_query_recovery_is_passive_and_does_not_schedule_initial_acquire() {
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    let summary = fixtures::session();
    state.replace_terminals(vec![summary]);
    events::apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::QueryCreation {
                request: yolo_hub_terminal_protocol::CreationQueryRequest::new(
                    yolo_hub_terminal_protocol::CreateShellRequest::new(
                        yolo_hub_terminal_protocol::CreateShellFields {
                            schema_version:
                                yolo_hub_terminal_protocol::TERMINAL_PROTOCOL_SCHEMA_VERSION,
                            request_id: uuid::Uuid::new_v4(),
                            creation_token: uuid::Uuid::new_v4(),
                            name: None,
                            cwd: fixtures::SESSION_DIRECTORY.into(),
                            size: Default::default(),
                        },
                    )
                    .unwrap(),
                ),
            },
            reply: Reply::Completed {},
        },
    );
    assert!(incoming.try_recv().is_err());
    assert!(state.initial_control.is_none());
}
