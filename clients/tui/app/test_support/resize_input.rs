use super::query_support::Fixture;
use crate::network::Job;
use ratatui::layout::Rect;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, ResizeRequest, TerminalEvent,
    TerminalEvent::Unreachable, TerminalSize, TerminalSnapshot, TerminalSnapshotFields,
};
use yolo_tui_render::PaneViewport;

pub(crate) const INITIAL_WIDTH: u16 = 80;
pub(crate) const RESIZED_WIDTH: u16 = 100;
pub(crate) const SECOND_RESIZED_WIDTH: u16 = 120;
pub(crate) const TERMINAL_HEIGHT: u16 = 24;
pub(crate) const VIEW_ORIGIN: u16 = 0;
pub(crate) const SNAPSHOT_SEQUENCE: u64 = 0;
pub(crate) const SNAPSHOT_DATA: &[u8] = b"";

pub(crate) fn viewport(address: yolo_hub_identity::TerminalAddress, width: u16) -> PaneViewport {
    PaneViewport {
        pane: Uuid::new_v4(),
        terminal: Some(address),
        outer: Rect::new(VIEW_ORIGIN, VIEW_ORIGIN, width, TERMINAL_HEIGHT),
        inner: Rect::new(VIEW_ORIGIN, VIEW_ORIGIN, width, TERMINAL_HEIGHT),
    }
}

pub(crate) fn resize_request(fixture: &mut Fixture) -> ResizeRequest {
    let Job::Call {
        command: Command::Resize { request },
        ..
    } = fixture.incoming.try_recv().expect("resize request")
    else {
        panic!("expected resize request")
    };
    request
}

pub(crate) fn snapshot(
    target: yolo_hub_identity::RuntimeTarget,
    size: TerminalSize,
) -> TerminalSnapshot {
    TerminalSnapshot::new(TerminalSnapshotFields {
        target,
        barrier_sequence: SNAPSHOT_SEQUENCE,
        size,
        data: SNAPSHOT_DATA.to_vec(),
    })
    .expect("valid resize snapshot")
}

pub(crate) fn attachment(
    target: yolo_hub_identity::RuntimeTarget,
    size: TerminalSize,
) -> (AttachRequest, AttachResult) {
    let request = AttachRequest::new(target, None).expect("snapshot attach request");
    let attachment = AttachResult::new(AttachResultFields {
        target,
        snapshot: Some(snapshot(target, size)),
        chunks: Vec::new(),
        history: None,
        next_sequence: yolo_hub_terminal_protocol::FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .expect("valid snapshot attachment");
    (request, attachment)
}

pub(crate) fn unreachable(target: yolo_hub_identity::RuntimeTarget) -> TerminalEvent {
    Unreachable { target }
}

pub(crate) fn resized_size(width: u16) -> TerminalSize {
    TerminalSize::new(width, TERMINAL_HEIGHT).expect("valid terminal size")
}
