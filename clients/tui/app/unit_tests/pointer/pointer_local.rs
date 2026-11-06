use super::fixtures;
use super::panes::*;
use crate::{Mode, pointer};
use crossterm::event::{MouseButton, MouseEventKind};
use yolo_hub_agent_protocol::{AgentSnapshot, Lifecycle};
use yolo_hub_terminal_protocol::{
    TerminalSize, TerminalSnapshot, TerminalSnapshotFields, TerminalSummary,
};
use yolo_tui_terminal::TerminalSurface;

#[test]
fn passive_or_resynchronizing_terminals_never_receive_mouse_input() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, ENABLE_MOUSE);
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    state.mark_resize_pending(second.fields().address);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    state.clear_all_resize_pending();
    state.writers.clear();
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn observer_scrollback_remains_local_even_when_the_source_requests_mouse_reporting() {
    let (mut state, _, second) = state();
    let mut content = Vec::new();
    for _ in 0..SCROLLBACK_LINES {
        content.extend_from_slice(LINE);
    }
    content.extend_from_slice(ENABLE_MOUSE);
    writable(&mut state, &second, &content);
    state.writers.clear();
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert!(
        state.surfaces[&second.fields().address]
            .screen()
            .scrollback()
            > 0
    );
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollDown, view.inner.x, view.inner.y),
    );
    assert_eq!(
        state.surfaces[&second.fields().address]
            .screen()
            .scrollback(),
        0
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn alternate_screen_wheel_sends_page_keys_when_the_program_did_not_request_mouse() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, ALT_SCREEN);
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), PAGE_UP);
    assert_eq!(
        state.surfaces[&second.fields().address]
            .screen()
            .scrollback(),
        0
    );

    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollDown, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), PAGE_DOWN);
}

#[test]
fn codex_wheel_opens_its_transcript_then_scrolls_it() {
    let (mut state, _, second) = state();
    bind_codex(&mut state, &second);
    writable(&mut state, &second, &codex_main());
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(
        bytes(incoming.try_recv().unwrap()),
        CODEX_OPEN_UP,
        "wheel-up opens Codex's transcript and scrolls a few lines"
    );
    assert_eq!(
        state.surfaces[&second.fields().address]
            .screen()
            .scrollback(),
        0,
        "the transcript is Codex-driven, not local scrollback"
    );

    rewrite_surface(&mut state, &second, CODEX_TRANSCRIPT);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_UP);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollDown, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_DOWN);
}

#[test]
fn codex_composer_adapts_wheel_without_agent_observation() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, &codex_main());
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_OPEN_UP);

    rewrite_surface(&mut state, &second, CODEX_TRANSCRIPT);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_UP);
}


include!("pointer_codex_fixtures.rs");
include!("pointer_codex_wheel.rs");
include!("pointer_geometry.rs");
