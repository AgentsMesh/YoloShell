use std::mem::size_of;

include!("../../../../../src/generated/scm_rights_descriptor_contract.rs");

// XNU accepts at most UIPC_MAX_CMSG_FD descriptors in one rights message.
pub const DESCRIPTOR_BYTES: usize = size_of::<libc::c_int>();
const CONTROL_DATA_BYTES: usize = DESCRIPTOR_BYTES * MAX_RECEIVED_DESCRIPTORS;
pub(crate) const CONTROL_BYTES: usize =
    unsafe { libc::CMSG_SPACE(CONTROL_DATA_BYTES as _) as usize };
const EMPTY_CONTROL_DATA_BYTES: u32 = 0;
pub(crate) const EMPTY_CONTROL_BYTES: usize =
    unsafe { libc::CMSG_LEN(EMPTY_CONTROL_DATA_BYTES) as usize };
pub const EXPECTED_IO_VECTOR_COUNT: libc::c_int = 1;
pub(super) const EXPECTED_RIGHT_COUNT: usize = 1;
pub(super) const EXPECTED_MARKER_BYTES: libc::ssize_t = 1;
pub(super) const SYSTEM_FAILURE: libc::ssize_t = -1;
pub(super) const FCNTL_FAILURE: libc::c_int = -1;
pub(super) const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;
pub(super) const POLL_READY: libc::c_int = 1;
pub(super) const POLL_TIMEOUT: libc::c_int = 0;
pub(super) const INFINITE_POLL_MILLISECONDS: libc::c_int = -1;
pub(super) const MINIMUM_POLL_MILLISECONDS: u128 = 1;
pub(super) const POISONED_DETAIL: &str = "local Unix stream is poisoned";
pub(super) const WAIT_TIMEOUT_DETAIL: &str = "local Unix stream read timed out";
pub(super) const UNEXPECTED_ANCILLARY_DETAIL: &str =
    "local Unix stream carried forbidden ancillary data";
pub(super) const INVALID_ANCILLARY_DETAIL: &str = "local Unix ancillary frame is invalid";
