use super::query_support::*;
use crate::{constants::QUEUE_CAPACITY, network::Job};
use std::time::Instant;

#[test]
fn queued_reply_lifetimes_end_when_writer_binding_subscription_or_control_is_retired() {
    let retire: &[fn(&mut Fixture)] = &[
        |f| {
            f.state.retire_writer(f.target.address);
        },
        |f| {
            f.replace_writer();
        },
        |f| {
            f.state.bind_focused(None).unwrap();
            f.state.sync_query_authority();
        },
        |f| {
            f.state.disconnected();
        },
        |f| {
            f.state.clear_observer_generation(f.target.address);
            f.state.sync_query_authority();
        },
        |f| {
            f.state.writers.get_mut(&f.target.address).unwrap().expires = Instant::now();
            f.state.sync_query_authority();
        },
        |f| {
            f.state.surfaces.remove(&f.target.address);
            f.state.sync_query_authority();
        },
    ];
    for retire in retire {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let request = fixture.queued();
        assert!(request.validity.as_ref().unwrap().is_current());
        retire(&mut fixture);
        assert!(!request.validity.unwrap().is_current());
        assert!(fixture.state.queries.pending.is_empty());
        fixture.no_jobs();
    }
}

#[test]
fn full_job_queue_discards_generated_reply_instead_of_replaying_it_later() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    assert!(
        fixture
            .jobs
            .try_send(Job::Refresh {
                request_id: uuid::Uuid::new_v4(),
            })
            .is_ok()
    );
    fixture.emit(DSR);
    assert!(fixture.state.queries.pending.is_empty());
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    fixture.emit(POSITION);
    fixture.no_jobs();
    fixture.emit(DSR);
    assert_eq!(input(&fixture.queued()).fields().data, STATUS_REPLY);
    fixture.no_jobs();
}

#[test]
fn reply_count_byte_budget_and_pending_budget_bound_floods_without_deferred_work() {
    for (query, expected) in [(DSR, STATUS_REPLY), (DA, ATTRIBUTE_REPLY)] {
        let mut fixture = Fixture::new();
        fixture.emit(&query.repeat(FLOOD_COUNT));
        let mut replies = Vec::new();
        while !fixture.incoming.is_empty() {
            replies.push(fixture.queued());
        }
        assert_eq!(
            replies.len(),
            MAX_REPLIES_PER_CHUNK.min(MAX_REPLY_BYTES_PER_CHUNK / expected.len())
        );
        assert!(
            replies
                .iter()
                .all(|reply| input(reply).fields().data == expected)
        );
        assert!(
            replies
                .iter()
                .map(|reply| input(reply).fields().data.len())
                .sum::<usize>()
                <= MAX_REPLY_BYTES_PER_CHUNK
        );
        fixture.emit(POSITION);
        fixture.no_jobs();
    }
    let mut fixture = Fixture::new();
    for _ in usize::default()..QUEUE_CAPACITY {
        fixture.emit(DSR);
        fixture.queued();
    }
    assert_eq!(fixture.state.queries.pending.len(), QUEUE_CAPACITY);
    fixture.emit(DSR);
    fixture.no_jobs();
    assert_eq!(fixture.state.queries.pending.len(), QUEUE_CAPACITY);
}

#[test]
fn a_closed_queue_cannot_leave_an_inflight_query_receipt() {
    let mut fixture = Fixture::new();
    fixture.incoming.close();
    fixture.emit(DSR);
    assert!(fixture.state.queries.pending.is_empty());
}
