pub(crate) const PROGRAM: &str = yolo_launch_contract::ProcessRole::FileHelper.display_name();
pub(crate) const OWNER_WATCH_THREAD: &str = "yolo-file-owner";
pub(crate) const EXPECTED_REQUEST: &str = "file helper requires one private request";
pub(crate) const OWNER_EVENTS: libc::c_short = libc::POLLIN | libc::POLLHUP | libc::POLLERR;
pub(crate) const OWNER_DESCRIPTOR_COUNT: libc::nfds_t = 1;
