use std::time::Duration;

pub(crate) const DISPLAY_DURATION: Duration = Duration::from_secs(8);
pub(crate) const SEPARATOR: &str = " · ";
pub(crate) const ALERT_LABEL: &str = "Notification";
pub(crate) const BELL: &[u8] = b"\x07";
