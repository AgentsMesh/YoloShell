use super::{fixtures, query_support::*};
use crate::{
    events,
    network::{ObservationUpdate, Update},
};
use std::time::Instant;
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_terminal_protocol::{LeaseGrant, TerminalSize};

#[test]
fn passive_invalid_or_unready_authority_cannot_generate_terminal_input() {
    let changes: &[fn(&mut Fixture)] = &[
        |f| {
            f.state.writers.clear();
        },
        |f| {
            f.state.principal.controller_instance_id = uuid::Uuid::new_v4();
        },
        |f| {
            f.state.control_generation = None;
        },
        |f| {
            f.state.writers.get_mut(&f.target.address).unwrap().expires = Instant::now();
        },
        |f| {
            f.state.mark_offline(f.target.address);
        },
        |f| {
            f.state.mark_ended(f.target.address, f.target.runtime_epoch);
        },
        |f| {
            f.state.bind_focused(None).unwrap();
        },
        |f| {
            f.state.clear_all_last_sizes();
        },
        |f| {
            f.state.mark_resize_pending(f.target.address);
        },
        |f| {
            f.state.mark_resync(f.target.address);
        },
        |f| {
            f.state.surfaces.clear();
        },
        |f| {
            f.state.set_last_size(
                f.target.address,
                TerminalSize::new(
                    TerminalSize::default().cols() + STEP as u16,
                    TerminalSize::default().rows(),
                )
                .unwrap(),
            );
        },
        |f| {
            let mut fields = *f.grant.fields();
            fields.target.runtime_epoch = RuntimeEpoch::new();
            f.state.writers.get_mut(&f.target.address).unwrap().grant =
                LeaseGrant::new(fields).unwrap();
        },
    ];
    for change in changes {
        let mut fixture = Fixture::new();
        change(&mut fixture);
        fixture.emit(DSR);
        fixture.no_jobs();
        assert!(fixture.state.queries.pending.is_empty());
    }
}

#[test]
fn stale_observer_or_foreign_runtime_events_cannot_reach_query_parser() {
    for wrong in [Wrong::Observer, Wrong::OuterTarget, Wrong::EventTarget] {
        let mut fixture = Fixture::new();
        let cursor = fixture.state.surfaces[&fixture.target.address].cursor();
        let foreign = fixtures::session().fields().runtime.unwrap().target;
        let event_target = if matches!(wrong, Wrong::EventTarget) {
            foreign
        } else {
            fixture.target
        };
        let event = yolo_hub_terminal_protocol::TerminalEvent::Output {
            chunk: yolo_hub_terminal_protocol::OutputChunk::new(
                yolo_hub_terminal_protocol::OutputChunkFields {
                    target: event_target,
                    sequence: cursor.after_sequence + STEP,
                    data: DSR.to_vec(),
                },
            )
            .unwrap(),
        };
        let target = if matches!(wrong, Wrong::OuterTarget) {
            foreign
        } else {
            fixture.target
        };
        let generation = if matches!(wrong, Wrong::Observer) {
            uuid::Uuid::new_v4()
        } else {
            fixture.observer
        };
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Observation {
                target,
                generation,
                update: ObservationUpdate::Terminal { event },
            },
        );
        fixture.no_jobs();
        assert_eq!(
            fixture.state.surfaces[&fixture.target.address].cursor(),
            cursor
        );
    }
}

enum Wrong {
    Observer,
    OuterTarget,
    EventTarget,
}
