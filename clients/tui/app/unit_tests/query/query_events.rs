use super::{fixtures, query_support::*};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{OutputChunk, OutputChunkFields, TerminalEvent};

#[test]
fn scoped_live_queries_send_exact_source_input_even_when_another_pane_is_focused() {
    let mut fixture = Fixture::new();
    let other = fixtures::session();
    let address = other.fields().address;
    fixture.state.terminals.insert(address, other);
    let pane = fixture
        .state
        .tab_layout_mut()
        .split(yolo_presentation_layout::Axis::Horizontal, Some(address))
        .unwrap();
    fixture.state.tab_layout_mut().focus(pane).unwrap();
    assert_eq!(fixture.state.focused_terminal(), Some(address));
    for (query, expected) in [
        (DSR, STATUS_REPLY),
        (CPR, CURSOR_REPLY),
        (DA, ATTRIBUTE_REPLY),
    ] {
        fixture.emit(query);
        let queued = fixture.queued();
        assert_eq!(queued.generation, fixture.state.control_generation);
        assert_eq!(queued.destination, fixture.target.address.hub_id);
        let request = input(&queued);
        assert_eq!(request.fields().grant, fixture.grant);
        assert_eq!(request.fields().data, expected);
        assert!(!request.fields().request_id.is_nil());
        let validity = queued.validity.as_ref().unwrap();
        assert!(validity.is_current());
        assert_eq!(
            validity.deadline(),
            fixture.state.writers[&fixture.target.address].expires
        );
        fixture.no_jobs();
    }
}

#[test]
fn unscoped_output_and_duplicate_or_gapped_observer_events_never_reply() {
    let mut fixture = Fixture::new();
    let event = fixture.event(DSR);
    crate::observation_events::terminal(&mut fixture.state, event);
    fixture.no_jobs();
    let current = fixture.state.surfaces[&fixture.target.address].cursor();
    for sequence in [current.after_sequence, current.after_sequence + STEP + STEP] {
        let event = TerminalEvent::Output {
            chunk: OutputChunk::new(OutputChunkFields {
                target: fixture.target,
                sequence,
                data: DSR.to_vec(),
            })
            .unwrap(),
        };
        fixture.scoped(crate::network::ObservationUpdate::Terminal { event });
        fixture.no_jobs();
    }
    assert!(!fixture.state.surfaces.contains_key(&fixture.target.address));
}

#[test]
fn ordinary_terminal_input_completion_is_not_mistaken_for_a_query_completion() {
    let mut fixture = Fixture::new();
    fixture.emit(DSR);
    let original = fixture.queued();
    let mut queued = original.clone();
    queued.validity = None;
    assert_eq!(
        crate::query_replies::completed(&mut fixture.state, &queued, None),
        Some(false)
    );
    queued = original;
    queued.command = Command::Hello {};
    assert_eq!(
        crate::query_replies::completed(&mut fixture.state, &queued, None),
        None
    );
}

#[test]
fn a_surface_gap_requests_a_resync_instead_of_leaving_an_empty_pane() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    let sequence = fixture.state.surfaces[&address].cursor().after_sequence + 2;
    let chunk = OutputChunk::new(OutputChunkFields {
        target: fixture.target,
        sequence,
        data: b"gap".to_vec(),
    })
    .unwrap();
    crate::query_replies::live(
        &mut fixture.state,
        &fixture.jobs,
        fixture.observer,
        TerminalEvent::Output { chunk },
    );
    assert!(fixture.state.is_resync(address));
    assert!(!fixture.state.surfaces.contains_key(&address));
}
