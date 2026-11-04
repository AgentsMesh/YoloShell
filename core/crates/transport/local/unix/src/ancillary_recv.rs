use super::ancillary_control::{
    AlignedControl, ReceivedDescriptors, received_descriptors, secure_all,
};
use super::constants::*;
use super::{InvalidAncillaryFrame, UnexpectedAncillaryData};
use std::io;
use std::os::fd::OwnedFd;

struct ReceivedMessage {
    bytes: libc::ssize_t,
    descriptors: ReceivedDescriptors,
    control_present: bool,
    control_truncated: bool,
}

pub(super) fn receive_rejecting(socket: libc::c_int, destination: &mut [u8]) -> io::Result<usize> {
    let _spawn_guard = yolo_process_spawn::waitable_child_spawn_guard()
        .map_err(|error| io::Error::other(error.to_string()))?;
    let received = receive(socket, destination)?;
    pause_after_recvmsg(socket);
    if received.control_present || received.control_truncated {
        let kind = secure_all(&received.descriptors.owned)
            .err()
            .map_or(io::ErrorKind::InvalidData, |error| error.kind());
        drop(received);
        return Err(io::Error::new(kind, UnexpectedAncillaryData));
    }
    usize::try_from(received.bytes).map_err(io::Error::other)
}

pub(super) fn receive_exact(
    socket: libc::c_int,
    expected_marker: u8,
    expected_count: usize,
) -> io::Result<Vec<OwnedFd>> {
    validate_count(expected_count)?;
    let _spawn_guard = yolo_process_spawn::waitable_child_spawn_guard()
        .map_err(|error| io::Error::other(error.to_string()))?;
    let mut marker = u8::default();
    let received = receive(socket, std::slice::from_mut(&mut marker))?;
    pause_after_recvmsg(socket);
    secure_all(&received.descriptors.owned)?;
    let control_expected = expected_count != usize::default();
    let valid = received.bytes == EXPECTED_MARKER_BYTES
        && marker == expected_marker
        && received.control_present == control_expected
        && !received.control_truncated
        && received.descriptors.frame_valid
        && received.descriptors.owned.len() == expected_count;
    if !valid {
        drop(received);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            InvalidAncillaryFrame,
        ));
    }
    Ok(received.descriptors.owned)
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
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT as _,
        msg_control: control.bytes.as_mut_ptr().cast(),
        msg_controllen: CONTROL_BYTES as _,
        msg_flags: libc::c_int::default(),
    };
    let bytes = unsafe { libc::recvmsg(socket, &mut message, receive_flags()) };
    if bytes == SYSTEM_FAILURE {
        return Err(io::Error::last_os_error());
    }
    super::ancillary_fail_stop::terminate_if_truncated(message.msg_flags);
    let descriptors = received_descriptors(&message, &control);
    Ok(ReceivedMessage {
        bytes,
        control_present: (message.msg_controllen as usize) != usize::default(),
        control_truncated: message.msg_flags & libc::MSG_CTRUNC != libc::c_int::default(),
        descriptors,
    })
}

#[cfg(all(target_os = "macos", any(test, feature = "yolo-test-support")))]
fn pause_after_recvmsg(socket: libc::c_int) {
    super::test_support::pause_after_recvmsg(socket);
}

#[cfg(not(all(target_os = "macos", any(test, feature = "yolo-test-support"))))]
const fn pause_after_recvmsg(_: libc::c_int) {}

#[cfg(target_os = "linux")]
const fn receive_flags() -> libc::c_int {
    libc::MSG_DONTWAIT | libc::MSG_CMSG_CLOEXEC
}

#[cfg(not(target_os = "linux"))]
const fn receive_flags() -> libc::c_int {
    libc::MSG_DONTWAIT
}

fn validate_count(count: usize) -> io::Result<()> {
    if count > MAX_RECEIVED_DESCRIPTORS {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            InvalidAncillaryFrame,
        ))
    } else {
        Ok(())
    }
}
