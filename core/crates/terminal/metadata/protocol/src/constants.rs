pub const MAX_TITLE_BYTES: usize = 512;
pub const MAX_NOTIFICATION_TITLE_BYTES: usize = 256;
pub const MAX_NOTIFICATION_BODY_BYTES: usize = 2048;
pub(crate) const INVALID_PRESENTATION: &str = "invalid terminal presentation";
pub(crate) const INVALID_NOTIFICATION: &str = "invalid terminal notification";
/// Bound for a non-authoritative terminal text read (visible screen only).
pub const MAX_TERMINAL_TEXT_BYTES: usize = 256 * 1024;
