mod callbacks;
mod constants;
mod snapshot;
mod text;

pub use callbacks::MetadataCallbacks;
pub use snapshot::title_from_snapshot;
pub use yolo_terminal_metadata_protocol::{
    MAX_NOTIFICATION_BODY_BYTES, MAX_NOTIFICATION_TITLE_BYTES, MAX_TITLE_BYTES, NotificationKind,
    TerminalNotification, TerminalPresentation,
};
