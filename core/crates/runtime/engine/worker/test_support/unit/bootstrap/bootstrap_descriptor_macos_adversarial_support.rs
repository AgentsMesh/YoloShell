use super::*;
use crate::process::MacosProcessCapabilityRelease;

pub(super) fn reject_and_close_many(count: usize) {
    let (manager, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let issuers = (0..count)
        .map(|_| MacosProcessCapabilityIssuer::create().unwrap())
        .collect::<Vec<_>>();
    let descriptors = issuers
        .iter()
        .map(MacosProcessCapabilityIssuer::writer_descriptor)
        .collect::<Vec<_>>();
    send_many(manager.as_raw_fd(), &descriptors).unwrap();
    assert!(receive_once(child.as_raw_fd()).is_err());
    for issuer in issuers {
        assert_released(issuer);
    }
}

pub(super) fn assert_released(issuer: MacosProcessCapabilityIssuer) {
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let identity = issuer.identity();
    let reader = issuer
        .commit_acknowledgement(identity.reader_handle(), identity.writer_handle())
        .unwrap();
    assert_eq!(
        reader.poll_release_for_test().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

pub(super) fn send_many(socket: libc::c_int, descriptors: &[libc::c_int]) -> Result<()> {
    let data_bytes = std::mem::size_of_val(descriptors);
    let control_bytes = unsafe { libc::CMSG_SPACE(data_bytes as _) as usize };
    let mut control = vec![u64::default(); control_bytes.div_ceil(std::mem::size_of::<u64>())];
    let mut marker = crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER;
    let mut vector = libc::iovec {
        iov_base: std::ptr::addr_of_mut!(marker).cast(),
        iov_len: std::mem::size_of_val(&marker),
    };
    let header = control.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(data_bytes as _);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        std::ptr::copy_nonoverlapping(
            descriptors.as_ptr(),
            libc::CMSG_DATA(header).cast(),
            descriptors.len(),
        );
    }
    let message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT,
        msg_control: control.as_mut_ptr().cast(),
        msg_controllen: control_bytes as _,
        msg_flags: libc::c_int::default(),
    };
    let sent = unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) };
    ensure_system_result(sent, FRAME_BYTES)
}
