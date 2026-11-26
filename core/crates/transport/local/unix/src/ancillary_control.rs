use super::constants::*;
use std::os::fd::{FromRawFd, OwnedFd};

#[repr(C, align(8))]
pub(super) struct AlignedControl {
    pub(super) bytes: [u8; CONTROL_BYTES],
}

pub(super) struct ReceivedDescriptors {
    pub(super) owned: Vec<OwnedFd>,
    pub(super) frame_valid: bool,
}

impl AlignedControl {
    pub(super) fn zeroed() -> Self {
        Self {
            bytes: [u8::default(); CONTROL_BYTES],
        }
    }
}

pub(super) fn received_descriptors(
    message: &libc::msghdr,
    control: &AlignedControl,
) -> ReceivedDescriptors {
    let length = (message.msg_controllen as usize).min(CONTROL_BYTES);
    let mut received = ReceivedDescriptors {
        owned: Vec::new(),
        frame_valid: message.msg_flags & libc::MSG_CTRUNC == libc::c_int::default(),
    };
    let mut offset = usize::default();
    while offset < length {
        if length - offset < EMPTY_CONTROL_BYTES {
            received.frame_valid = false;
            break;
        }
        let header = unsafe { &*control.bytes.as_ptr().add(offset).cast::<libc::cmsghdr>() };
        let header_length = header.cmsg_len as usize;
        if header_length < EMPTY_CONTROL_BYTES {
            received.frame_valid = false;
            break;
        }
        let rights = header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_RIGHTS;
        let data_bytes = header_length - EMPTY_CONTROL_BYTES;
        let available = length.saturating_sub(offset + EMPTY_CONTROL_BYTES);
        if rights {
            own_rights(
                control,
                offset,
                data_bytes.min(available),
                &mut received.owned,
            );
        }
        if header_length > length - offset {
            received.frame_valid = false;
            break;
        }
        received.frame_valid &= rights && data_bytes % DESCRIPTOR_BYTES == usize::default();
        let padded = unsafe { libc::CMSG_SPACE(data_bytes as _) as usize };
        if padded == usize::default() || offset + padded > length {
            received.frame_valid = false;
            break;
        }
        offset += padded;
    }
    received.frame_valid &= offset == length;
    received
}

fn own_rights(
    control: &AlignedControl,
    offset: usize,
    data_bytes: usize,
    owned: &mut Vec<OwnedFd>,
) {
    for index in 0..(data_bytes / DESCRIPTOR_BYTES) {
        let raw = unsafe {
            std::ptr::read_unaligned(
                control
                    .bytes
                    .as_ptr()
                    .add(offset + EMPTY_CONTROL_BYTES + index * DESCRIPTOR_BYTES)
                    .cast::<libc::c_int>(),
            )
        };
        if raw >= libc::c_int::default() {
            owned.push(unsafe { OwnedFd::from_raw_fd(raw) });
        }
    }
}

pub(super) fn secure_all(descriptors: &[OwnedFd]) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    for descriptor in descriptors {
        let raw = descriptor.as_raw_fd();
        let flags = unsafe { libc::fcntl(raw, libc::F_GETFD) };
        if flags == FCNTL_FAILURE
            || unsafe { libc::fcntl(raw, libc::F_SETFD, flags | libc::FD_CLOEXEC) } == FCNTL_FAILURE
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}
