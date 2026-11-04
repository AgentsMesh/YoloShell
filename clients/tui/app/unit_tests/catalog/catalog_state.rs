use super::query_support::Fixture;
use crate::{
    Mode,
    query_state::{ActiveQueryScope, PendingReply},
    terminal_actions,
};
use std::time::{Duration, Instant};
use uuid::Uuid;
use yolo_hub_client_channels::RequestScope;
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::{InputFields, InputRequest, TerminalSummary};
use yolo_tui_terminal::QueryScope;

const PENDING_QUERY_TTL: Duration = Duration::from_secs(5);

#[test]
fn catalog_runtime_replacement_discards_every_old_control_surface() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    fixture.state.mode = Mode::Terminal;
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"input".to_vec())
    ));
    let _ = fixture.incoming.try_recv();
    fixture
        .state
        .inputs
        .interrupted
        .insert(address, fixture.grant);
    fixture.state.inputs.resume_requested.insert(address);
    fixture.state.inputs.overflowed.insert(address);
    fixture.state.mark_resize_pending(address);
    fixture.state.mark_geometry_syncing(address);
    fixture.state.mark_resync(address);
    fixture.state.mark_offline(address);
    fixture
        .state
        .mark_ended(address, fixture.target.runtime_epoch);
    fixture
        .state
        .set_last_size(address, yolo_hub_terminal_protocol::TerminalSize::default());
    fixture.state.queries.scopes.insert(
        address,
        ActiveQueryScope {
            parser: QueryScope::new(Uuid::new_v4()).unwrap(),
            queue: RequestScope::default(),
            grant: fixture.grant,
            observer: fixture.observer,
            control: fixture.state.control_generation.unwrap(),
        },
    );
    let validity = RequestScope::default().validity_until(Instant::now() + PENDING_QUERY_TTL);
    let request = InputRequest::new(InputFields {
        request_id: Uuid::new_v4(),
        grant: fixture.grant,
        data: b"query".to_vec(),
    })
    .unwrap();
    fixture.state.queries.pending.insert(
        request.fields().request_id,
        PendingReply {
            request,
            generation: fixture.state.control_generation.unwrap(),
            validity,
        },
    );

    let mut fields = fixture.state.terminals[&address].fields().clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
    let replacement = TerminalSummary::new(fields).unwrap();
    fixture.state.replace_terminals(vec![replacement]);

    assert!(!fixture.state.writers.contains_key(&address));
    assert!(!fixture.state.inputs.streams.contains_key(&address));
    assert!(!fixture.state.inputs.interrupted.contains_key(&address));
    assert!(!fixture.state.inputs.resume_requested.contains(&address));
    assert!(!fixture.state.inputs.overflowed.contains(&address));
    assert!(fixture.state.inputs.scope.is_none());
    assert!(fixture.state.inputs.pending.is_empty());
    assert!(fixture.state.queries.scopes.is_empty());
    assert!(fixture.state.queries.pending.is_empty());
    assert!(!fixture.state.is_resize_pending(address));
    assert!(!fixture.state.is_geometry_syncing(address));
    assert!(!fixture.state.is_resync(address));
    assert!(!fixture.state.is_offline(address));
    assert!(!fixture.state.ended_epoch(address).is_some());
    assert!(!fixture.state.last_size(address).is_some());
    assert!(!fixture.state.surfaces.contains_key(&address));
    assert!(fixture.state.control.permits(fixture.target));
}
