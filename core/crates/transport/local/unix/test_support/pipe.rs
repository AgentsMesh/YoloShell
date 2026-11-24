use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const SYSCALL_SUCCESS: libc::c_int = 0;

pub struct Pipe {
    pub reader: OwnedFd,
    pub writer: OwnedFd,
}

pub fn pipe() -> Pipe {
    let _spawn = yolo_process_spawn::waitable_child_spawn_guard().unwrap();
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        SYSCALL_SUCCESS
    );
    let pipe = Pipe {
        reader: unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_READER_INDEX]) },
        writer: unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) },
    };
    for descriptor in [&pipe.reader, &pipe.writer] {
        assert_eq!(
            unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) },
            SYSCALL_SUCCESS,
        );
    }
    pipe
}
