#![cfg(unix)]

use yolo_relay::TerminalSize;

#[path = "../timing.rs"]
mod timing;
use timing::{POLL_INTERVAL, RELAY_TIMEOUT};

#[path = "../../relay_dimensions.rs"]
mod dimensions;
