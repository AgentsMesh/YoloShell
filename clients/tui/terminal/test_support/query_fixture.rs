use yolo_hub_terminal_protocol::{OutputChunk, OutputChunkFields, TerminalEvent};
use yolo_tui_terminal::{QueryScope, TerminalSurface};

#[path = "query_constants.rs"]
mod constants;
pub use constants::*;

pub fn scope() -> QueryScope {
    QueryScope::new(uuid::Uuid::new_v4()).unwrap()
}

pub fn event(surface: &TerminalSurface, bytes: &[u8]) -> TerminalEvent {
    TerminalEvent::Output {
        chunk: OutputChunk::new(OutputChunkFields {
            target: surface.cursor().target,
            sequence: surface.cursor().after_sequence
                + yolo_hub_terminal_protocol::FIRST_OUTPUT_SEQUENCE,
            data: bytes.to_vec(),
        })
        .unwrap(),
    }
}

pub fn apply(
    surface: &mut TerminalSurface,
    bytes: &[u8],
    scope: Option<QueryScope>,
) -> Vec<Vec<u8>> {
    let event = event(surface, bytes);
    surface.apply_live_event(&event, scope).unwrap()
}
