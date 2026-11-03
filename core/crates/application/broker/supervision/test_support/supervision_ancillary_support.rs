use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const DESCRIPTOR_BYTES: usize = std::mem::size_of::<libc::c_int>();
const CONTROL_BYTES: usize = unsafe { libc::CMSG_SPACE(DESCRIPTOR_BYTES as _) as usize };
const EXPECTED_IO_VECTOR_COUNT: libc::c_int = 1;
const EXPECTED_MARKER_BYTES: libc::ssize_t = 1;
const SYSTEM_SUCCESS: libc::c_int = 0;
const SYSTEM_FAILURE: libc::c_int = -1;
pub(super) const ANCILLARY_MARKER: u8 = b'S';

#[repr(C)]
struct AlignedControl {
    bytes: [u8; CONTROL_BYTES],
    _alignment: [libc::cmsghdr; 0],
}

pub(super) struct Pipe {
    pub(super) reader: OwnedFd,
    pub(super) writer: OwnedFd,
}

pub(super) fn pipe() -> Pipe {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        SYSTEM_SUCCESS
    );
    Pipe {
        reader: unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_READER_INDEX]) },
        writer: unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) },
    }
}

pub(super) fn send_right(socket: libc::c_int, descriptor: libc::c_int) {
    let mut marker = ANCILLARY_MARKER;
    let mut control = AlignedControl {
        bytes: [u8::default(); CONTROL_BYTES],
        _alignment: [],
    };
    let header = control.bytes.as_mut_ptr().cast::<libc::cmsghdr>();
    unsafe {
        (*header).cmsg_len = libc::CMSG_LEN(DESCRIPTOR_BYTES as _);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        std::ptr::write_unaligned(libc::CMSG_DATA(header).cast(), descriptor);
    }
    let mut vector = libc::iovec {
        iov_base: std::ptr::addr_of_mut!(marker).cast(),
        iov_len: std::mem::size_of_val(&marker),
    };
    let message = libc::msghdr {
        msg_name: std::ptr::null_mut(),
        msg_namelen: libc::socklen_t::default(),
        msg_iov: std::ptr::addr_of_mut!(vector),
        msg_iovlen: EXPECTED_IO_VECTOR_COUNT,
        msg_control: control.bytes.as_mut_ptr().cast(),
        msg_controllen: CONTROL_BYTES as _,
        msg_flags: libc::c_int::default(),
    };
    assert_eq!(
        unsafe { libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) },
        EXPECTED_MARKER_BYTES
    );
}

pub(super) fn assert_reader_eof(reader: &OwnedFd) {
    let flags = unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) };
    assert_ne!(flags, SYSTEM_FAILURE);
    assert_ne!(
        unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) },
        SYSTEM_FAILURE
    );
    let mut marker = u8::default();
    assert_eq!(
        unsafe {
            libc::read(
                reader.as_raw_fd(),
                std::ptr::addr_of_mut!(marker).cast(),
                std::mem::size_of_val(&marker),
            )
        },
        libc::ssize_t::default()
    );
}
