#[path = "ancillary_control.rs"]
mod ancillary_control;
#[path = "ancillary_fail_stop.rs"]
mod ancillary_fail_stop;
#[path = "ancillary_recv.rs"]
mod ancillary_recv;
#[path = "ancillary_send.rs"]
mod ancillary_send;
#[path = "async_stream.rs"]
mod async_stream;
#[path = "blocking_stream.rs"]
mod blocking_stream;
#[path = "blocking_wait.rs"]
mod blocking_wait;
#[path = "constants.rs"]
mod constants;
#[cfg(target_os = "macos")]
#[path = "control_macos.rs"]
mod control_macos;
#[cfg(target_os = "macos")]
#[path = "recv_macos.rs"]
mod recv_macos;
#[path = "rights.rs"]
pub mod rights;

pub use async_stream::LocalUnixStream;
pub use blocking_stream::BlockingLocalUnixStream;

#[derive(Debug)]
pub struct UnexpectedAncillaryData;

impl std::fmt::Display for UnexpectedAncillaryData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(constants::UNEXPECTED_ANCILLARY_DETAIL)
    }
}

impl std::error::Error for UnexpectedAncillaryData {}

#[derive(Debug)]
pub(crate) struct InvalidAncillaryFrame;

impl std::fmt::Display for InvalidAncillaryFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(constants::INVALID_ANCILLARY_DETAIL)
    }
}

impl std::error::Error for InvalidAncillaryFrame {}

pub(crate) fn is_ancillary_failure(error: &std::io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.is::<UnexpectedAncillaryData>())
        || error
            .get_ref()
            .is_some_and(|source| source.is::<InvalidAncillaryFrame>())
}

#[cfg(target_os = "macos")]
pub mod macos {
    pub use super::constants::{DESCRIPTOR_BYTES, EXPECTED_IO_VECTOR_COUNT};
    pub use super::control_macos::AlignedControl;
    #[cfg(any(test, feature = "yolo-test-support"))]
    pub use super::control_macos::received_descriptors;
    pub use super::recv_macos::receive_exactly_one_right;
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/rights_tests.rs"]
mod rights_tests;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/stream_tests.rs"]
mod stream_tests;
#[cfg(all(target_os = "macos", any(test, feature = "yolo-test-support")))]
#[path = "../test_support/receive_pause.rs"]
pub mod test_support;
