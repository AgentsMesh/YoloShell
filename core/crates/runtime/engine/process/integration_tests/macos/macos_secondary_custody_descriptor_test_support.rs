const ORIGINAL_DESCRIPTOR_REMAINS_OPEN_DETAIL: &str =
    "rejected custody import retained an original descriptor";
const UNEXPECTED_DESCRIPTOR_QUERY_ERROR: &str =
    "querying a rejected custody descriptor failed unexpectedly";

fn assert_exact_descriptor_order(
    descriptors: [BorrowedFd<'_>; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
) {
    let mut pty_name = [u8::default(); PTY_NAME_CAPACITY];
    assert_ne!(
        unsafe {
            libc::ioctl(
                descriptors[PTY_MASTER_DESCRIPTOR_INDEX].as_raw_fd(),
                libc::TIOCPTYGNAME.into(),
                pty_name.as_mut_ptr(),
            )
        },
        SYSCALL_FAILURE
    );
    let reader_flags = unsafe {
        libc::fcntl(
            descriptors[PROCESS_READER_DESCRIPTOR_INDEX].as_raw_fd(),
            libc::F_GETFL,
        )
    };
    assert_ne!(reader_flags, FCNTL_FAILURE);
    assert_eq!(reader_flags & libc::O_ACCMODE, libc::O_RDONLY);
    for descriptor in descriptors {
        let flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) };
        assert_ne!(flags, FCNTL_FAILURE);
        assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DescriptorIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
    mode: libc::mode_t,
    raw_device: libc::dev_t,
}

#[derive(Clone, Copy)]
struct DescriptorProbe {
    raw: libc::c_int,
    identity: DescriptorIdentity,
}

fn descriptor_probes(
    descriptors: [&OwnedFd; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
) -> [DescriptorProbe; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] {
    descriptors.map(DescriptorProbe::capture)
}

impl DescriptorProbe {
    fn capture(descriptor: &OwnedFd) -> Self {
        let raw = descriptor.as_raw_fd();
        Self {
            raw,
            identity: descriptor_identity(raw).unwrap(),
        }
    }
}

fn assert_closed(descriptors: [DescriptorProbe; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT]) {
    for descriptor in descriptors {
        match descriptor_identity(descriptor.raw) {
            Ok(current) => assert_ne!(
                current, descriptor.identity,
                "{ORIGINAL_DESCRIPTOR_REMAINS_OPEN_DETAIL}: {}",
                descriptor.raw
            ),
            Err(error) => assert_eq!(
                error.raw_os_error(),
                Some(libc::EBADF),
                "{UNEXPECTED_DESCRIPTOR_QUERY_ERROR}: {error}"
            ),
        }
    }
}

fn descriptor_identity(raw: libc::c_int) -> std::io::Result<DescriptorIdentity> {
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(raw, status.as_mut_ptr()) } == SYSCALL_FAILURE {
        return Err(std::io::Error::last_os_error());
    }
    let status = unsafe { status.assume_init() };
    Ok(DescriptorIdentity {
        device: status.st_dev,
        inode: status.st_ino,
        mode: status.st_mode,
        raw_device: status.st_rdev,
    })
}
