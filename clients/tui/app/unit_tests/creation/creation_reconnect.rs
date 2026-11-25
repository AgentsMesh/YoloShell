use super::{creation_support::*, fixtures};
use crate::{
    events,
    network::{Job, Update},
};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_terminal_protocol::CreateShellResult;
use yolo_presentation_layout::Axis;

#[test]
fn admitted_creation_survives_fast_reconnect_without_restoring_writer_authority() {
    let mut fixture = Fixture::new();
    let request = fixture.start();
    let writer = fixtures::grant(&fixture.state.terminals[&fixture.expected]);
    fixture
        .state
        .record_grant(writer, std::time::Instant::now());
    reconnect(&mut fixture);
    assert!(fixture.state.writers.is_empty());
    assert!(
        fixture
            .state
            .creations
            .pending
            .contains_key(&create(&request).fields().request_id)
    );
    let current = fixture.state.control_generation;
    assert_ne!(current, request.generation);
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.tab_layout().focused(), fixture.pane);
    assert_eq!(fixture.state.selected, Some(created.address));
    assert!(fixture.state.creations.pending.is_empty());
    assert!(fixture.state.writers.is_empty());
    assert_eq!(fixture.state.control_generation, current);
    assert_finalized(&mut fixture, &request, created, true);
    fixture.finish(request.clone(), created);
    assert_finalized(&mut fixture, &request, created, false);
}

#[test]
fn reconnected_creation_finalizes_without_overwriting_rebound_or_removed_destinations() {
    for changed in [Changed::Rebound, Changed::ClosedPane, Changed::RemovedTab] {
        let mut fixture = Fixture::new();
        let request = fixture.start();
        reconnect(&mut fixture);
        let replacement = fixtures::session().fields().address;
        match changed {
            Changed::Rebound => fixture
                .state
                .tab_layout_mut()
                .bind(fixture.pane, Some(replacement))
                .unwrap(),
            Changed::ClosedPane => {
                fixture
                    .state
                    .tab_layout_mut()
                    .split(Axis::Horizontal, Some(replacement))
                    .unwrap();
                fixture.state.tab_layout_mut().close(fixture.pane).unwrap();
            }
            Changed::RemovedTab => {
                fixture.switch_tab();
                fixture.state.tabs.remove(fixture.tab).unwrap();
            }
        }
        let layout = fixture.state.tab_layout().clone();
        let tabs = fixture.state.tab_snapshot();
        let created = fixture.created();
        fixture.finish(request.clone(), created);
        assert_eq!(*fixture.state.tab_layout(), layout);
        assert_eq!(fixture.state.tab_snapshot(), tabs);
        assert_ne!(fixture.original_binding(), Some(created.address));
        assert!(fixture.state.writers.is_empty());
        assert!(fixture.state.creations.pending.is_empty());
        assert_finalized(&mut fixture, &request, created, true);
    }
}

enum Changed {
    Rebound,
    ClosedPane,
    RemovedTab,
}

pub(super) fn reconnect(fixture: &mut Fixture) {
    let previous = fixture.state.control_generation;
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Unreachable {
            generation: previous,
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    assert!(fixture.state.control_generation.is_none());
    assert!(fixture.state.writers.is_empty());
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(crate::client_capability_support::ready(Uuid::new_v4())),
    );
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    assert!(fixture.incoming.try_recv().is_err());
}

pub(super) fn assert_finalized(
    fixture: &mut Fixture,
    admitted: &ControlRequest,
    created: CreateShellResult,
    expected: bool,
) {
    let mut current = admitted.clone();
    current.generation = fixture.state.control_generation;
    fixture.assert_finalize(&current, created, expected);
}
