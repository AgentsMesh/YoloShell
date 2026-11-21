mod constants;
pub use constants::*;
pub use yolo_terminal_metadata_protocol::{
    MAX_NOTIFICATION_BODY_BYTES, MAX_NOTIFICATION_TITLE_BYTES, MAX_TITLE_BYTES,
    TerminalPresentation,
};

pub fn presentation() -> TerminalPresentation {
    use yolo_terminal_metadata_protocol::{NotificationKind, TerminalNotification};
    TerminalPresentation {
        title: Some(TITLE.into()),
        notification_sequence: FIRST_NOTIFICATION,
        notification: Some(TerminalNotification {
            kind: NotificationKind::Desktop,
            title: TITLE.into(),
            body: BODY.into(),
        }),
    }
}
