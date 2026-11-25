use super::query_support::*;
use crate::network::ObservationUpdate;
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, FIRST_OUTPUT_SEQUENCE, OutputChunk,
    OutputChunkFields,
};

#[test]
fn snapshot_and_replay_queries_are_passive_and_retire_preexisting_queued_replies() {
    for restored_snapshot in [false, true] {
        let mut fixture = Fixture::new();
        fixture.emit(DSR);
        let queued = fixture.queued();
        let cursor = fixture.state.surfaces[&fixture.target.address].cursor();
        let next = cursor.after_sequence + FIRST_OUTPUT_SEQUENCE;
        let attachment = AttachResult::new(if restored_snapshot {
            AttachResultFields {
                target: fixture.target,
                snapshot: Some(snapshot(fixture.target, next, DSR)),
                chunks: Vec::new(),
                history: None,
                next_sequence: next + FIRST_OUTPUT_SEQUENCE,
                resynchronized: true,
            }
        } else {
            AttachResultFields {
                target: fixture.target,
                snapshot: None,
                chunks: vec![
                    OutputChunk::new(OutputChunkFields {
                        target: fixture.target,
                        sequence: next,
                        data: DSR.to_vec(),
                    })
                    .unwrap(),
                ],
                history: None,
                next_sequence: next + FIRST_OUTPUT_SEQUENCE,
                resynchronized: false,
            }
        })
        .unwrap();
        fixture.scoped(ObservationUpdate::Attached {
            request: AttachRequest::new(fixture.target, Some(cursor)).unwrap(),
            attachment,
        });
        fixture.no_jobs();
        assert!(!queued.validity.unwrap().is_current());
        assert!(fixture.state.queries.pending.is_empty());
        fixture.emit(DSR);
        assert_eq!(input(&fixture.queued()).fields().data, STATUS_REPLY);
    }
}

#[test]
fn replayed_query_prefix_cannot_complete_when_the_subscription_becomes_live() {
    let mut fixture = Fixture::new();
    let cursor = fixture.state.surfaces[&fixture.target.address].cursor();
    let attachment = AttachResult::new(AttachResultFields {
        target: fixture.target,
        snapshot: Some(snapshot(
            fixture.target,
            cursor.after_sequence,
            PARTIAL_QUERY,
        )),
        chunks: Vec::new(),
        history: None,
        next_sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    fixture.scoped(ObservationUpdate::Attached {
        request: AttachRequest::new(fixture.target, None).unwrap(),
        attachment,
    });
    fixture.emit(QUERY_FINAL);
    fixture.no_jobs();
    fixture.emit(DSR);
    assert_eq!(input(&fixture.queued()).fields().data, STATUS_REPLY);
}
