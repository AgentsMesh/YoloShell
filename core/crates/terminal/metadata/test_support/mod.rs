#![allow(dead_code, unused_imports)]

pub use yolo_terminal_metadata::{
    MAX_NOTIFICATION_BODY_BYTES, MAX_NOTIFICATION_TITLE_BYTES, MAX_TITLE_BYTES, MetadataCallbacks,
    NotificationKind, TerminalPresentation,
};

mod constants;
pub use constants::*;

pub fn parser() -> vt100::Parser<MetadataCallbacks> {
    vt100::Parser::new_with_callbacks(ROWS, COLUMNS, SCROLLBACK, MetadataCallbacks::default())
}

pub fn osc(command: &[u8], body: &[u8], ending: &[u8]) -> Vec<u8> {
    [OSC_PREFIX, command, OSC_SEPARATOR, body, ending].concat()
}

pub fn presentation(bytes: &[u8]) -> TerminalPresentation {
    let mut parser = parser();
    parser.process(bytes);
    parser.callbacks().presentation().clone()
}
