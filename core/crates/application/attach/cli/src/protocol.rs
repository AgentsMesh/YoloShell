use yolo_relay::RelayStatus;


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StatusEventKind {
    ControlAcquired,
    ReadOnly,
    ControlLost,
    HostStarted,
    SessionResumed,
    SessionEnding,
    SessionExited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StatusFrame {
    pub(super) event: StatusEventKind,
    pub(super) holder_id: Option<String>,
}

impl StatusFrame {
    pub fn native(self) -> yolo_hub_relay_control_protocol::Status {
        use yolo_hub_relay_control_protocol::{Status, StatusKind};
        Status { kind: match self.event {
            StatusEventKind::ControlAcquired => StatusKind::ControlAcquired,
            StatusEventKind::ReadOnly => StatusKind::ReadOnly,
            StatusEventKind::ControlLost => StatusKind::ControlLost,
            StatusEventKind::HostStarted => StatusKind::HostStarted,
            StatusEventKind::SessionResumed => StatusKind::SessionResumed,
            StatusEventKind::SessionEnding => StatusKind::SessionEnding,
            StatusEventKind::SessionExited => StatusKind::SessionExited,
        }, holder_id: self.holder_id }
    }
}

impl From<RelayStatus> for StatusFrame {
    fn from(status: RelayStatus) -> Self {
        let (event, holder_id) = match status {
            RelayStatus::ControlAcquired => (StatusEventKind::ControlAcquired, None),
            RelayStatus::ReadOnly { holder_id } => (StatusEventKind::ReadOnly, holder_id),
            RelayStatus::ControlLost { holder_id } => (StatusEventKind::ControlLost, holder_id),
            RelayStatus::HostStarted { .. } => (StatusEventKind::HostStarted, None),
            RelayStatus::SessionResumed => (StatusEventKind::SessionResumed, None),
            RelayStatus::SessionEnding => (StatusEventKind::SessionEnding, None),
            RelayStatus::SessionExited => (StatusEventKind::SessionExited, None),
        };
        Self { event, holder_id }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/protocol_tests.rs"]
mod tests;
