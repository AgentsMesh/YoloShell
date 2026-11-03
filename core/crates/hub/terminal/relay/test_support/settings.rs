use super::fixtures::*;
use yolo_hub_terminal_relay::{ControlIntent, Options};
pub fn options(socket: std::path::PathBuf, intent: ControlIntent) -> Options {
    Options {
        socket,
        entry: hub(),
        target: target(),
        principal: principal(),
        intent,
        size: None,
    }
}
