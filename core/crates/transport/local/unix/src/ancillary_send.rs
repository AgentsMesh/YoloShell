use super::InvalidAncillaryFrame;
use super::ancillary_control::AlignedControl;
use super::constants::*;
use std::io;
use std::mem::size_of_val;
use std::os::fd::RawFd;

pub(super) fn send_exact(socket: RawFd, marker: u8, descriptors: &[RawFd]) -> io::Result<()> {
    validate(descriptors)?;
    let mut marker = marker;
    let mut vector = libc::iovec {
        iov_base: std::ptr::addr_of_mut!(marker).cast(),
        iov_len: size_of_val(&marker),
    };
    let mut control = AlignedControl::zeroed();
    let mut message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT as _,
        msg_control: std::ptr::null_mut(),
        msg_controllen: Default::default(),
        msg_flags: libc::c_int::default(),
    };
    attach_rights(&mut message, &mut control, descriptors);
    let sent = unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) };
    if sent == SYSTEM_FAILURE {
        return Err(io::Error::last_os_error());
    }
    if sent != EXPECTED_MARKER_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            InvalidAncillaryFrame,
        ));
    }
    Ok(())
}

fn attach_rights(message: &mut libc::msghdr, control: &mut AlignedControl, descriptors: &[RawFd]) {
    if descriptors.is_empty() {
        return;
    }
    let data_bytes = size_of_val(descriptors);
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(data_bytes as _) as _;
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        std::ptr::copy_nonoverlapping(
            descriptors.as_ptr(),
            libc::CMSG_DATA(header).cast(),
            descriptors.len(),
        );
        message.msg_controllen = libc::CMSG_SPACE(data_bytes as _) as _;
    }
    message.msg_control = control.bytes.as_mut_ptr().cast();
}

fn validate(descriptors: &[RawFd]) -> io::Result<()> {
    let valid_count = descriptors.len() <= MAX_RECEIVED_DESCRIPTORS;
    let valid_descriptors = descriptors.iter().all(|descriptor| *descriptor >= 0);
    if valid_count && valid_descriptors {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            InvalidAncillaryFrame,
        ))
    }
}
