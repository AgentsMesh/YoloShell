use super::*;
use std::os::fd::RawFd;

const INVALID_DESCRIPTOR: RawFd = -1;

#[test]
fn state_anchor_rejects_an_invalid_directory_descriptor() {
    assert!(BrokerStateAnchor::capture_descriptor(INVALID_DESCRIPTOR).is_err());
}
