use crate::local_unix::LocalUnixStream;
#[cfg(test)]
use crate::local_unix::LocalUnixStream as UnixStream;
use crate::local_unix::macos::{AlignedControl, DESCRIPTOR_BYTES, EXPECTED_IO_VECTOR_COUNT};
use crate::process::{MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer};
use anyhow::{Context, Result, ensure};
use std::mem::size_of_val;
use std::os::fd::AsRawFd;
use tokio::time::{Instant, timeout_at};

use super::super::constants::BOOTSTRAP_GATE_TIMEOUT_CONTEXT;

pub(crate) const DESCRIPTOR_SEND_CONTEXT: &str = "send bootstrap process capability writer";
pub(crate) const DESCRIPTOR_RECEIVE_CONTEXT: &str = "receive bootstrap process capability writer";
pub(crate) const DESCRIPTOR_FRAME_DETAIL: &str =
    "bootstrap process capability descriptor frame is invalid";
pub(crate) const SYSTEM_FAILURE: libc::ssize_t = -1;
pub(crate) const FRAME_BYTES: libc::ssize_t = 1;

#[path = "bootstrap_descriptor_macos_receive.rs"]
mod receive_descriptor;
use receive_descriptor::receive_once;

pub(crate) async fn send_before(
    stream: &LocalUnixStream,
    issuer: &MacosProcessCapabilityIssuer,
    deadline: Instant,
) -> Result<()> {
    timeout_at(deadline, send(stream, issuer))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    Ok(())
}

pub(crate) async fn receive_before(
    stream: &LocalUnixStream,
    deadline: Instant,
) -> Result<MacosProcessCapabilityChildWriter> {
    timeout_at(deadline, receive(stream))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)?
}

pub(crate) async fn send(
    stream: &LocalUnixStream,
    issuer: &MacosProcessCapabilityIssuer,
) -> Result<()> {
    stream
        .async_io(tokio::io::Interest::WRITABLE, || {
            readiness_result(send_once(
                stream.as_raw_fd(),
                issuer.writer_descriptor(),
                crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
            ))
        })
        .await
        .context(DESCRIPTOR_SEND_CONTEXT)?
        .context(DESCRIPTOR_SEND_CONTEXT)
}

pub(crate) async fn receive(stream: &LocalUnixStream) -> Result<MacosProcessCapabilityChildWriter> {
    stream
        .async_io(tokio::io::Interest::READABLE, || {
            readiness_result(receive_once(stream.as_raw_fd()))
        })
        .await
        .context(DESCRIPTOR_RECEIVE_CONTEXT)?
        .context(DESCRIPTOR_RECEIVE_CONTEXT)
}

pub(crate) fn readiness_result<T>(result: Result<T>) -> std::io::Result<Result<T>> {
    if result.as_ref().is_err_and(|error| is_would_block(error)) {
        Err(std::io::ErrorKind::WouldBlock.into())
    } else {
        Ok(result)
    }
}

pub(crate) fn send_once(
    socket: libc::c_int,
    descriptor: libc::c_int,
    mut marker: u8,
) -> Result<()> {
    let mut vector = libc::iovec {
        iov_base: std::ptr::addr_of_mut!(marker).cast(),
        iov_len: size_of_val(&marker),
    };
    let mut control = AlignedControl::zeroed();
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(DESCRIPTOR_BYTES as _);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        std::ptr::write_unaligned(libc::CMSG_DATA(header).cast(), descriptor);
    }
    let message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT,
        msg_control: control.bytes.as_mut_ptr().cast(),
        msg_controllen: unsafe { libc::CMSG_SPACE(DESCRIPTOR_BYTES as _) } as _,
        msg_flags: libc::c_int::default(),
    };
    let sent = unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) };
    ensure_system_result(sent, FRAME_BYTES)
}

#[cfg(test)]
pub(crate) fn send_writer_once_for_test(
    socket: libc::c_int,
    descriptor: libc::c_int,
) -> Result<()> {
    send_once(
        socket,
        descriptor,
        crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    )
}

#[cfg(test)]
pub(crate) fn receive_writer_once_for_test(
    socket: libc::c_int,
) -> Result<MacosProcessCapabilityChildWriter> {
    receive_once(socket)
}

pub(crate) fn ensure_system_result(actual: libc::ssize_t, expected: libc::ssize_t) -> Result<()> {
    if actual == SYSTEM_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DESCRIPTOR_FRAME_DETAIL);
    }
    ensure!(actual == expected, DESCRIPTOR_FRAME_DETAIL);
    Ok(())
}

pub(crate) fn is_would_block(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::WouldBlock)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/bootstrap/bootstrap_descriptor_macos_adversarial_support.rs"]
mod adversarial_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_descriptor_macos_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_descriptor_macos_cmsg_tests.rs"]
mod cmsg_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_descriptor_macos_receive_spawn_tests.rs"]
mod receive_spawn_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_descriptor_macos_tests.rs"]
mod tests;
