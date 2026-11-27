use std::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_tui_terminal_control::{Attempt, Controller};

pub const BEFORE_DEADLINE: std::time::Duration = std::time::Duration::from_nanos(1);

pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::new(),
            terminal_id: TerminalId::new(),
        },
        runtime_epoch: RuntimeEpoch::new(),
    }
}

pub fn controller() -> (Controller, Instant) {
    (Controller::new(target()), Instant::now())
}

pub fn begin(controller: &mut Controller, now: Instant) -> Attempt {
    controller.request_write(now);
    controller
        .begin(Uuid::new_v4(), Uuid::new_v4(), now)
        .unwrap()
}
