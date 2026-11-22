use super::*;
use std::os::fd::{FromRawFd, IntoRawFd, OwnedFd};

const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const INVALID_CMSG_TYPE: libc::c_int = libc::c_int::MAX;
const CLOSE_SUCCESS: libc::c_int = 0;
const MALFORMED_TRAILING_BYTES: usize = 1;

#[test]
fn rights_are_owned_when_invalid_cmsg_precedes_them() {
    assert_rights_closed_across_invalid_order(true);
}

#[test]
fn rights_are_owned_when_invalid_cmsg_follows_them() {
    assert_rights_closed_across_invalid_order(false);
}

#[test]
fn complete_rights_prefix_is_owned_when_payload_has_a_remainder() {
    let (raw, _readers) = raw_pipe_writers();
    let rights_bytes = std::mem::size_of_val(&raw);
    let malformed_bytes = rights_bytes + MALFORMED_TRAILING_BYTES;
    let control_length = unsafe { libc::CMSG_SPACE(malformed_bytes as _) as usize };
    let mut control = AlignedControl::zeroed();
    write_header(&mut control, usize::default(), libc::SCM_RIGHTS, &raw);
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(malformed_bytes as _);
    }
    let received = receive_descriptor::received_descriptors(
        &message_with_control_length(control_length),
        &control,
    );
    assert!(!received.frame_valid);
    assert_eq!(received.owned.len(), raw.len());
    assert_owned_descriptors_close(received.owned);
}

#[test]
fn truncated_rights_owns_only_the_complete_available_prefix() {
    assert_oversized_rights_owns_available_prefix(libc::MSG_CTRUNC);
}

#[test]
fn inconsistent_rights_length_owns_the_complete_available_prefix() {
    assert_oversized_rights_owns_available_prefix(libc::c_int::default());
}

fn assert_oversized_rights_owns_available_prefix(message_flags: libc::c_int) {
    let (raw, _readers) = raw_pipe_writers();
    let available_bytes = std::mem::size_of_val(&raw) + MALFORMED_TRAILING_BYTES;
    let declared_bytes = available_bytes + DESCRIPTOR_BYTES;
    let control_length = unsafe { libc::CMSG_LEN(available_bytes as _) as usize };
    let mut control = AlignedControl::zeroed();
    write_header(&mut control, usize::default(), libc::SCM_RIGHTS, &raw);
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(declared_bytes as _);
    }
    let mut message = message_with_control_length(control_length);
    message.msg_flags = message_flags;

    let received = receive_descriptor::received_descriptors(&message, &control);
    assert!(!received.frame_valid);
    assert_eq!(received.owned.len(), raw.len());
    assert_owned_descriptors_close(received.owned);
}

fn assert_rights_closed_across_invalid_order(invalid_first: bool) {
    let (raw, _readers) = raw_pipe_writers();
    let rights_bytes = std::mem::size_of_val(&raw);
    let invalid_space = unsafe { libc::CMSG_SPACE(u32::default()) as usize };
    let rights_space = unsafe { libc::CMSG_SPACE(rights_bytes as _) as usize };
    let mut control = AlignedControl::zeroed();
    let (invalid_offset, rights_offset) = if invalid_first {
        (usize::default(), invalid_space)
    } else {
        (rights_space, usize::default())
    };
    write_header(&mut control, invalid_offset, INVALID_CMSG_TYPE, &[]);
    write_header(&mut control, rights_offset, libc::SCM_RIGHTS, &raw);
    let message = message_with_control_length(invalid_space + rights_space);

    let received = receive_descriptor::received_descriptors(&message, &control);
    assert!(!received.frame_valid);
    assert_eq!(received.owned.len(), raw.len());
    assert_owned_descriptors_close(received.owned);
}

fn raw_pipe_writers() -> ([libc::c_int; PIPE_ENDPOINT_COUNT], Vec<OwnedFd>) {
    let mut raw = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    let mut readers = Vec::new();
    for slot in &mut raw {
        let mut pipe = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
        assert_eq!(
            unsafe { libc::pipe(pipe.as_mut_ptr()) },
            libc::c_int::default()
        );
        readers.push(unsafe { OwnedFd::from_raw_fd(pipe[PIPE_READER_INDEX]) });
        *slot = unsafe { OwnedFd::from_raw_fd(pipe[PIPE_WRITER_INDEX]) }.into_raw_fd();
    }
    (raw, readers)
}

fn assert_owned_descriptors_close(descriptors: Vec<OwnedFd>) {
    for descriptor in descriptors {
        let raw = descriptor.into_raw_fd();
        assert_eq!(unsafe { libc::close(raw) }, CLOSE_SUCCESS);
    }
}

fn write_header(
    control: &mut AlignedControl,
    offset: usize,
    kind: libc::c_int,
    descriptors: &[libc::c_int],
) {
    let data_bytes = std::mem::size_of_val(descriptors);
    let header = unsafe {
        control
            .bytes
            .as_mut_ptr()
            .add(offset)
            .cast::<libc::cmsghdr>()
    };
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(data_bytes as _);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = kind;
        std::ptr::copy_nonoverlapping(
            descriptors.as_ptr(),
            libc::CMSG_DATA(header).cast(),
            descriptors.len(),
        );
    }
}

fn message_with_control_length(length: usize) -> libc::msghdr {
    libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::null_mut(),
        msg_iovlen: libc::c_int::default(),
        msg_control: std::ptr::null_mut(),
        msg_controllen: length as _,
        msg_flags: libc::c_int::default(),
    }
}
