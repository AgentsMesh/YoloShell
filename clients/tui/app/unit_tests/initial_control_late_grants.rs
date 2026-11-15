use super::{creation_support::Fixture, fixtures};
use crate::{
    Mode, events,
    network::{Job, Update},
};
use crossterm::event::{Event, KeyCode};
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalSummary};
use yolo_presentation_layout::Axis;

const INVENTORY_PREFIX_KEY: char = 's';

pub(super) fn pending_acquire() -> (Fixture, ControlRequest, LeaseGrant) {
    let mut fixture = Fixture::new();
    let create = fixture.start();
    let result = fixture.created();
    fixture.finish(create.clone(), result);
    fixture.assert_finalize(&create, result, true);
    let mut fields = fixtures::session().fields().clone();
    fields.address = result.address;
    fields.runtime.as_mut().unwrap().target.address = result.address;
    let session = TerminalSummary::new(fields).unwrap();
    fixture.state.replace_terminals(vec![session.clone()]);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    let Job::InitialAcquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    let mut grant = *fixtures::grant(&session).fields();
    grant.principal = request.principal;
    let grant = LeaseGrant::new(grant).unwrap();
    (
        fixture,
        ControlRequest {
            validity: Some(validity),
            generation: Some(generation),
            destination: origin,
            command: Command::Lease { request },
        },
        grant,
    )
}

pub(super) fn finish(fixture: &mut Fixture, request: ControlRequest, grant: LeaseGrant) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result: Ok(Reply::Lease { grant }),
            connected: true,
        }),
    );
}

fn assert_released(fixture: &mut Fixture, request: &ControlRequest, grant: LeaseGrant) {
    assert!(fixture.state.initial_control.is_none());
    assert!(
        !fixture
            .state
            .writers
            .contains_key(&grant.fields().target.address)
    );
    assert!(matches!(fixture.incoming.try_recv().unwrap(), Job::Call {
        generation, origin, command: Command::Release { grant: actual },
    } if generation == request.generation && origin == request.destination && actual == grant));
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn current_initial_acquire_records_the_confirmed_writer_once() {
    let (mut fixture, request, grant) = pending_acquire();
    finish(&mut fixture, request, grant);
    assert_eq!(
        fixture.state.writers[&grant.fields().target.address].grant,
        grant
    );
    assert!(fixture.state.initial_control.is_none());
    assert_eq!(fixture.state.mode, Mode::Terminal);
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn navigation_releases_the_exact_late_initial_grant_without_returning_to_terminal() {
    let (mut fixture, request, grant) = pending_acquire();
    fixture.state.prefix = true;
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyCode::Char(INVENTORY_PREFIX_KEY).into()),
    );
    assert!(!request.validity.as_ref().unwrap().is_current());
    assert_eq!(fixture.state.mode, Mode::Inventory);
    finish(&mut fixture, request.clone(), grant);
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert_released(&mut fixture, &request, grant);
}

#[test]
fn replacing_the_original_pane_releases_only_its_late_initial_grant() {
    let (mut fixture, request, grant) = pending_acquire();
    let replacement = fixtures::session();
    let replacement_grant = fixtures::grant(&replacement);
    let address = replacement.fields().address;
    fixture.state.terminals.insert(address, replacement);
    let pane = fixture
        .state
        .tab_layout_mut()
        .split(Axis::Horizontal, Some(address))
        .unwrap();
    fixture.state.tab_layout_mut().close(fixture.pane).unwrap();
    fixture
        .state
        .record_grant(replacement_grant, std::time::Instant::now());
    assert!(request.validity.as_ref().unwrap().is_current());
    finish(&mut fixture, request.clone(), grant);
    assert_eq!(fixture.state.tab_layout().focused(), pane);
    assert_eq!(fixture.state.focused_terminal(), Some(address));
    assert_eq!(fixture.state.writers[&address].grant, replacement_grant);
    assert_released(&mut fixture, &request, grant);
}

#[test]
fn rebinding_the_same_pane_cannot_accept_its_old_initial_grant() {
    let (mut fixture, request, grant) = pending_acquire();
    fixture
        .state
        .tab_layout_mut()
        .bind(fixture.pane, Some(fixture.expected))
        .unwrap();
    assert!(request.validity.as_ref().unwrap().is_current());
    finish(&mut fixture, request.clone(), grant);
    assert_eq!(fixture.state.tab_layout().focused(), fixture.pane);
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert_released(&mut fixture, &request, grant);
}

#[test]
fn full_release_queue_drops_local_initial_authority_without_reacquiring() {
    let (mut fixture, request, grant) = pending_acquire();
    crate::initial_control::user_event(&mut fixture.state);
    let refresh = uuid::Uuid::new_v4();
    while fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: refresh,
        })
        .is_ok()
    {}
    finish(&mut fixture, request, grant);
    assert!(fixture.state.initial_control.is_none());
    assert!(fixture.state.writers.is_empty());
    assert_eq!(fixture.state.status, crate::constants::QUEUE_FULL_LABEL);
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Refresh { request_id } if request_id == refresh));
    }
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert!(fixture.incoming.try_recv().is_err());
    assert!(fixture.state.initial_control.is_none());
    assert!(fixture.state.writers.is_empty());
}
