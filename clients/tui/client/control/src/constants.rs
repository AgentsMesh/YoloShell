use std::time::Duration;
pub(crate) use yolo_tui_client_control_protocol::{
    CONTROL_DESCRIPTOR_NAME, MAX_CONTROL_FRAME_BYTES,
};
pub(crate) const CONTROL_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
pub(crate) const CONTROL_FRAME_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
pub(crate) const CONTROL_DESCRIPTOR_MAX_BYTES: u64 = MAX_CONTROL_FRAME_BYTES as u64;
pub(crate) const CONTROL_READ_SENTINEL: u64 = 1;
pub(crate) const CONTROL_OWNER_LOCK: &str = ".yolo-control.lock";
pub(crate) const CONTROL_NAMESPACE: &str = "TUI client control";
pub(crate) const CONTROL_PEER_UID_ERROR: &str =
    "terminal client control peer is not the current user";
pub(crate) const CONTROL_PROTOCOL_ERROR: &str = "terminal client control frame is invalid";
pub(crate) const CONTROL_DESCRIPTOR_ERROR: &str = "terminal client control descriptor is invalid";
pub(crate) const CONTROL_DETACH_REJECTED: &str = "terminal client detach request was rejected";
pub(crate) const CONTROL_BUSY: &str = "terminal client control already has an owner";
pub(crate) const CONTROL_DEADLINE: &str = "terminal client did not detach before its deadline";
pub(crate) const CONTROL_OWNER_FAILED: &str =
    "terminal client control service stopped unexpectedly";
pub(crate) const CONTROL_FINISH_FAILED: &str = "terminal client did not finish detaching";
