use super::{object, volume_uuid};
use crate::Error;
use std::os::fd::AsFd;

#[test]
fn non_filesystem_descriptor_cannot_supply_persistent_apfs_attributes() {
    let (socket, peer) = std::os::unix::net::UnixStream::pair().unwrap();
    for descriptor in [socket.as_fd(), peer.as_fd()] {
        let uuid = volume_uuid(descriptor)
            .err()
            .expect("socket must not supply volume UUID");
        let object = object(descriptor)
            .err()
            .expect("socket must not supply persistent file ID");
        for error in [uuid, object] {
            match error {
                Error::Io { source, .. } => assert!(source.raw_os_error().is_some()),
                Error::Unsupported(_) => {}
                other => {
                    panic!("kernel lookup failure must preserve its classification: {other:?}")
                }
            }
        }
    }
}
