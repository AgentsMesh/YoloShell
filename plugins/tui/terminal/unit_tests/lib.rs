#[path = "../test_support/fixtures.rs"]
mod fixtures;

use fixtures::*;
use yolo_hub_terminal_protocol::*;
use yolo_tui_terminal::TerminalSurface;

#[test]
fn snapshot_then_contiguous_attachment_updates_the_real_vt_screen() {
    let initial = snapshot();
    let mut surface = TerminalSurface::new(&initial);
    assert_eq!(surface.screen().contents().as_bytes(), CONTENT);
    let request = AttachRequest::new(initial.fields().target, Some(surface.cursor())).unwrap();
    let result = append(&initial);
    surface.apply(request, &result).unwrap();
    assert_eq!(surface.screen().contents(), "hello world");
    assert_eq!(
        surface.cursor().after_sequence,
        result.fields().next_sequence - FIRST_OUTPUT_SEQUENCE
    );
    assert_eq!(surface.apply(request, &result), Err(PublicError::OutputGap));
    surface.set_scrollback(usize::MAX);
    assert!(surface.screen().scrollback() <= yolo_tui_terminal::SCROLLBACK_ROWS);
}

#[test]
fn foreign_runtime_and_rollback_do_not_mutate_the_surface() {
    let initial = snapshot();
    let mut surface = TerminalSurface::new(&initial);
    let other = snapshot();
    let foreign = AttachRequest::new(other.fields().target, None).unwrap();
    assert_eq!(
        surface.apply(foreign, &append(&other)),
        Err(PublicError::TargetMismatch)
    );
    let request = AttachRequest::new(initial.fields().target, Some(surface.cursor())).unwrap();
    surface.apply(request, &append(&initial)).unwrap();
    let before = surface.screen().contents();
    let reset = AttachResult::new(AttachResultFields {
        target: initial.fields().target,
        snapshot: Some(initial.clone()),
        chunks: Vec::new(),
        history: None,
        next_sequence: initial.fields().barrier_sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    assert_eq!(
        surface.apply(
            AttachRequest::new(initial.fields().target, None).unwrap(),
            &reset
        ),
        Err(PublicError::OutputGap)
    );
    assert_eq!(surface.screen().contents(), before);
}

#[test]
fn state_formatted_seed_restores_input_modes() {
    let initial = snapshot();
    let mut parser = vt100::Parser::new(ROWS, COLUMNS, 0);
    parser.process(b"\x1b[?1h\x1b[?2004h\x1b[?1000h\x1b[?1006h");
    let seed = TerminalSnapshot::new(TerminalSnapshotFields {
        target: initial.fields().target,
        barrier_sequence: initial.fields().barrier_sequence,
        size: initial.fields().size,
        data: parser.screen().state_formatted(),
    })
    .unwrap();
    let surface = TerminalSurface::new(&seed);
    assert!(surface.screen().application_cursor());
    assert!(surface.screen().bracketed_paste());
    assert_eq!(
        surface.screen().mouse_protocol_mode(),
        vt100::MouseProtocolMode::PressRelease
    );
    assert_eq!(
        surface.screen().mouse_protocol_encoding(),
        vt100::MouseProtocolEncoding::Sgr
    );
}

#[test]
fn history_replay_restores_scrollback_modes_and_cursor() {
    let initial = snapshot();
    let mut data = String::new();
    for index in 0..200 {
        data.push_str(&format!("line-{index:03}\r\n"));
    }
    data.push_str("\x1b[?1h\x1b[?2004h");
    let history = TerminalHistory::new(TerminalHistoryFields {
        target: initial.fields().target,
        barrier_sequence: 42,
        size: TerminalSize::new(20, 5).unwrap(),
        data: data.into_bytes(),
    })
    .unwrap();
    let mut surface = TerminalSurface::from_history(&history);
    assert!(surface.screen().application_cursor());
    assert!(surface.screen().bracketed_paste());
    assert_eq!(surface.cursor().after_sequence, 42);
    let contents = surface.screen().contents();
    assert!(contents.contains("line-199"));
    surface.set_scrollback(usize::MAX);
    assert!(surface.screen().scrollback() > 0);

    let result = AttachResult::new(AttachResultFields {
        target: initial.fields().target,
        snapshot: None,
        chunks: Vec::new(),
        history: Some(history),
        next_sequence: 43,
        resynchronized: false,
    })
    .unwrap();
    let mut surface = TerminalSurface::new(&initial);
    surface
        .apply(
            AttachRequest::history(initial.fields().target, None).unwrap(),
            &result,
        )
        .unwrap();
    assert!(surface.screen().application_cursor());
    assert_eq!(surface.cursor().after_sequence, 42);
}

#[test]
fn resynchronization_accepts_empty_caught_up_response() {
    let initial = snapshot();
    let mut surface = TerminalSurface::new(&initial);
    let result = AttachResult::new(AttachResultFields {
        target: initial.fields().target,
        snapshot: Some(initial.clone()),
        chunks: Vec::new(),
        history: None,
        next_sequence: initial.fields().barrier_sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap();
    surface
        .apply(
            AttachRequest::new(initial.fields().target, None).unwrap(),
            &result,
        )
        .unwrap();
    let empty = AttachResult::new(AttachResultFields {
        target: initial.fields().target,
        snapshot: None,
        chunks: Vec::new(),
        history: None,
        next_sequence: initial.fields().barrier_sequence + FIRST_OUTPUT_SEQUENCE,
        resynchronized: false,
    })
    .unwrap();
    let request = AttachRequest::new(initial.fields().target, Some(surface.cursor())).unwrap();
    surface.apply(request, &empty).unwrap();
    assert_eq!(surface.screen().contents().as_bytes(), CONTENT);
}
