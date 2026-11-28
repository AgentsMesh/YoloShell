use super::constants::*;
use std::os::fd::{FromRawFd, OwnedFd};

#[repr(C)]
pub struct AlignedControl {
    pub bytes: [u8; CONTROL_BYTES],
    _alignment: [libc::cmsghdr; 0],
}

impl AlignedControl {
    pub fn zeroed() -> Self {
        Self {
            bytes: [u8::default(); CONTROL_BYTES],
            _alignment: [],
        }
    }
}

pub struct ReceivedDescriptors {
    pub owned: Vec<OwnedFd>,
    pub frame_valid: bool,
}

pub fn received_descriptors(
    message: &libc::msghdr,
    control: &AlignedControl,
) -> ReceivedDescriptors {
    let reported_length = message.msg_controllen as usize;
    let control_length = reported_length.min(control.bytes.len());
    let mut received = ReceivedDescriptors {
        owned: Vec::new(),
        frame_valid: reported_length == control_length
            && message.msg_flags & libc::MSG_CTRUNC == libc::c_int::default(),
    };
    let mut offset = usize::default();
    while offset + std::mem::size_of::<libc::cmsghdr>() <= control_length {
        let header = unsafe { &*control.bytes.as_ptr().add(offset).cast::<libc::cmsghdr>() };
        let header_length = header.cmsg_len as usize;
        if header_length < EMPTY_CONTROL_BYTES {
            received.frame_valid = false;
            break;
        }
        let rights = header.cmsg_level == libc::SOL_SOCKET && header.cmsg_type == libc::SCM_RIGHTS;
        let data_bytes = header_length - EMPTY_CONTROL_BYTES;
        let available_bytes = control_length.saturating_sub(offset + EMPTY_CONTROL_BYTES);
        if header_length > control_length - offset {
            if rights {
                own_rights(
                    control,
                    offset,
                    data_bytes.min(available_bytes),
                    &mut received.owned,
                );
            }
            received.frame_valid = false;
            break;
        }
        if rights {
            own_rights(control, offset, data_bytes, &mut received.owned);
        }
        received.frame_valid &= rights && data_bytes % DESCRIPTOR_BYTES == usize::default();
        let padded = unsafe { libc::CMSG_SPACE(data_bytes as _) as usize };
        if padded == usize::default() || offset + padded > control_length {
            received.frame_valid = false;
            break;
        }
        offset += padded;
    }
    received.frame_valid &= offset == control_length;
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
