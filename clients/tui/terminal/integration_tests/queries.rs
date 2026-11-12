use crate::{fixtures::*, query::*};
use yolo_hub_terminal_protocol::{OutputChunk, OutputChunkFields, PublicError, TerminalEvent};
use yolo_tui_terminal::TerminalSurface;

#[test]
fn live_queries_follow_the_exact_terminal_cursor_and_preserve_scope_through_heartbeat() {
    let mut surface = TerminalSurface::new(&snapshot());
    let scope = Some(scope());
    assert!(apply(&mut surface, PARTIAL_CPR, scope).is_empty());
    let heartbeat = TerminalEvent::Heartbeat {
        cursor: surface.cursor(),
    };
    assert!(
        surface
            .apply_live_event(&heartbeat, scope)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        apply(&mut surface, CPR_FINAL, scope),
        [SNAPSHOT_CURSOR_REPLY.to_vec()]
    );
    assert_eq!(
        apply(&mut surface, ORIGIN_QUERY, scope),
        [ORIGIN_REPLY.to_vec()]
    );
}

#[test]
fn passive_or_new_writer_and_explicit_retirement_do_not_complete_old_queries() {
    for transition in [
        Transition::Passive,
        Transition::Replacement,
        Transition::Retire,
    ] {
        let mut surface = TerminalSurface::new(&snapshot());
        let mut authority = Some(scope());
        assert!(apply(&mut surface, PARTIAL_CPR, authority).is_empty());
        match transition {
            Transition::Passive => authority = None,
            Transition::Replacement => authority = Some(scope()),
            Transition::Retire => surface.reset_query_scope(),
        }
        assert!(apply(&mut surface, CPR_FINAL, authority).is_empty());
        assert_eq!(
            apply(&mut surface, DSR, Some(scope())),
            [STATUS_REPLY.to_vec()]
        );
    }
}

enum Transition {
    Passive,
    Replacement,
    Retire,
}

#[test]
fn wrong_runtime_and_output_gap_leave_screen_unchanged_and_retire_query_authority() {
    for foreign in [false, true] {
        let mut surface = TerminalSurface::new(&snapshot());
        let scope = Some(scope());
        assert!(apply(&mut surface, PARTIAL_CPR, scope).is_empty());
        let cursor = surface.cursor();
        let before = surface.screen().contents();
        let invalid = TerminalEvent::Output {
            chunk: OutputChunk::new(OutputChunkFields {
                target: if foreign {
                    snapshot().fields().target
                } else {
                    cursor.target
                },
                sequence: cursor.after_sequence + QUERY_GAP,
                data: DSR.to_vec(),
            })
            .unwrap(),
        };
        assert_eq!(
            surface.apply_live_event(&invalid, scope),
            Err(if foreign {
                PublicError::TargetMismatch
            } else {
                PublicError::OutputGap
            })
        );
        assert_eq!(surface.cursor(), cursor);
        assert_eq!(surface.screen().contents(), before);
        assert!(apply(&mut surface, CPR_FINAL, scope).is_empty());
    }
}

#[test]
fn exit_and_quiet_heartbeat_clear_partial_query_authority() {
    for exited in [false, true] {
        let mut surface = TerminalSurface::new(&snapshot());
        let scope = Some(scope());
        assert!(apply(&mut surface, PARTIAL_CPR, scope).is_empty());
        let event = if exited {
            TerminalEvent::Exited {
                cursor: surface.cursor(),
                code: None,
                signal: None,
            }
        } else {
            TerminalEvent::Heartbeat {
                cursor: surface.cursor(),
            }
        };
        if exited {
            assert!(surface.apply_live_event(&event, scope).unwrap().is_empty());
        } else {
            surface.apply_event(&event).unwrap();
        }
        assert!(apply(&mut surface, CPR_FINAL, scope).is_empty());
    }
}
