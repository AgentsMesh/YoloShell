use super::constants::*;
use super::control_macos::{AlignedControl, ReceivedDescriptors, received_descriptors, secure_all};
use super::InvalidAncillaryFrame;
use std::io;
use std::os::fd::OwnedFd;

struct ReceivedMessage {
    bytes: libc::ssize_t,
    descriptors: ReceivedDescriptors,
    ancillary_present: bool,
}

pub fn receive_exactly_one_right(socket: libc::c_int, expected_marker: u8) -> io::Result<OwnedFd> {
    let _spawn_guard = yolo_process_spawn::waitable_child_spawn_guard()
        .map_err(|error| io::Error::other(error.to_string()))?;
    let mut marker = u8::default();
    let received = receive(socket, std::slice::from_mut(&mut marker))?;
    secure_all(&received.descriptors.owned)?;
    let valid = received.bytes == EXPECTED_MARKER_BYTES
        && marker == expected_marker
        && received.ancillary_present
        && received.descriptors.frame_valid
        && received.descriptors.owned.len() == EXPECTED_RIGHT_COUNT;
    if !valid {
        drop(received);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            InvalidAncillaryFrame,
        ));
    }
    match received.descriptors.owned.into_iter().next() {
        Some(descriptor) => Ok(descriptor),
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            InvalidAncillaryFrame,
        )),
    }
}

fn receive(socket: libc::c_int, destination: &mut [u8]) -> io::Result<ReceivedMessage> {
    let mut vector = libc::iovec {
        iov_base: destination.as_mut_ptr().cast(),
        iov_len: destination.len(),
    };
    let mut control = AlignedControl::zeroed();
    let mut message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT,
        msg_control: control.bytes.as_mut_ptr().cast(),
        msg_controllen: CONTROL_BYTES as _,
        msg_flags: libc::c_int::default(),
    };
    let bytes = unsafe { libc::recvmsg(socket, &mut message, libc::MSG_DONTWAIT) };
    if bytes == SYSTEM_FAILURE {
        return Err(io::Error::last_os_error());
    }
    #[cfg(any(test, feature = "yolo-test-support"))]
    super::test_support::pause_after_recvmsg(socket);
    let descriptors = received_descriptors(&message, &control);
    Ok(ReceivedMessage {
        bytes,
        ancillary_present: (message.msg_controllen as usize) != usize::default()
            || message.msg_flags & libc::MSG_CTRUNC != libc::c_int::default(),
        descriptors,
    })
}
