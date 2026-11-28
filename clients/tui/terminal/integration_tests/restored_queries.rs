use crate::{fixtures::*, query::*};
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, FIRST_OUTPUT_SEQUENCE, OutputChunk,
    OutputChunkFields, TerminalSnapshot, TerminalSnapshotFields,
};
use yolo_tui_terminal::TerminalSurface;

#[test]
fn snapshot_and_replay_queries_never_reach_the_next_live_reply_batch() {
    let initial = snapshot();
    let restored = TerminalSnapshot::new(TerminalSnapshotFields {
        data: DSR.to_vec(),
        ..initial.fields().clone()
    })
    .unwrap();
    let mut surface = TerminalSurface::new(&restored);
    assert_eq!(
        apply(&mut surface, DSR, Some(scope())),
        [STATUS_REPLY.to_vec()]
    );
    let cursor = surface.cursor();
    let result = AttachResult::new(AttachResultFields {
        target: cursor.target,
        snapshot: None,
        resynchronized: false,
        chunks: vec![
            OutputChunk::new(OutputChunkFields {
                target: cursor.target,
                sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE,
                data: DSR.to_vec(),
            })
            .unwrap(),
        ],
        history: None,
        next_sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE + FIRST_OUTPUT_SEQUENCE,
    })
    .unwrap();
    surface
        .apply(
            AttachRequest::new(cursor.target, Some(cursor)).unwrap(),
            &result,
        )
        .unwrap();
    assert_eq!(
        apply(&mut surface, DSR, Some(scope())),
        [STATUS_REPLY.to_vec()]
    );
}

#[test]
fn replay_partial_sequence_and_snapshot_reset_do_not_gain_live_reply_authority() {
    let initial = snapshot();
    let mut surface = TerminalSurface::new(&initial);
    let mut replay = append(&initial).fields().clone();
    let mut chunk = replay.chunks.remove(usize::default()).fields().clone();
    chunk.data = PARTIAL_CPR.to_vec();
    replay.chunks.push(OutputChunk::new(chunk).unwrap());
    surface
        .apply(
            AttachRequest::new(initial.fields().target, Some(surface.cursor())).unwrap(),
            &AttachResult::new(replay).unwrap(),
        )
        .unwrap();
    assert!(apply(&mut surface, CPR_FINAL, Some(scope())).is_empty());
    assert_eq!(
        apply(&mut surface, DSR, Some(scope())),
        [STATUS_REPLY.to_vec()]
    );
    let authority = Some(scope());
    assert!(apply(&mut surface, PARTIAL_CPR, authority).is_empty());
    let cursor = surface.cursor();
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        barrier_sequence: cursor.after_sequence,
        data: PARTIAL_CPR.to_vec(),
        ..initial.fields().clone()
    })
    .unwrap();
    let result = AttachResult::new(AttachResultFields {
        target: cursor.target,
        snapshot: Some(snapshot),
        chunks: Vec::new(),
        resynchronized: true,
        history: None,
        next_sequence: cursor.after_sequence + FIRST_OUTPUT_SEQUENCE,
    })
    .unwrap();
    surface
        .apply(
            AttachRequest::new(cursor.target, Some(cursor)).unwrap(),
            &result,
        )
        .unwrap();
    assert!(apply(&mut surface, CPR_FINAL, authority).is_empty());
}
