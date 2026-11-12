use crate::process::ProcessIdentity;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
#[path = "../../test_support/linux/linux_kernel_devpts.rs"]
mod descriptor;

const PTY_NAME_BYTES: usize = 128;
pub(super) const MAX_REUSE_ALLOCATIONS: usize = 64;
const WRITER_SIGNAL: &[u8] = b"g";
pub const STALE_PAYLOAD: &[u8] = b"yolo-stale-pty-injection";
const CHILD_SUCCESS: libc::c_int = 0;
const CHILD_FAILURE: libc::c_int = 91;
const WAIT_ANY_OPTIONS: libc::c_int = 0;

pub struct PtyPair {
    pub master: File,
    slave: File,
    pub path: PathBuf,
}

impl PtyPair {
    pub fn open() -> Self {
        let mut master = -1;
        let mut slave = -1;
        let mut name = [libc::c_char::default(); PTY_NAME_BYTES];
        let result = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                name.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        assert_eq!(result, 0, "open a real devpts pair");
        let path = unsafe { CStr::from_ptr(name.as_ptr()) };
        Self {
            master: descriptor::adopt(master),
            slave: descriptor::adopt(slave),
            path: PathBuf::from(path.to_str().expect("UTF-8 devpts path")),
        }
    }

    pub fn device(&self) -> u64 {
        self.slave.metadata().expect("stat devpts slave").rdev()
    }

    pub fn make_master_nonblocking(&self) {
        let descriptor = self.master.as_raw_fd();
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(
            unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) },
            0
        );
    }

    pub fn slave_descriptor(&self) -> RawFd {
        self.slave.as_raw_fd()
    }
}

pub fn allocate_reused(path: &PathBuf) -> (PtyPair, Vec<PtyPair>) {
    let mut retained = Vec::new();
    for _ in 0..MAX_REUSE_ALLOCATIONS {
        let candidate = PtyPair::open();
        if candidate.path == *path {
            return (candidate, retained);
        }
        retained.push(candidate);
    }
    panic!("devpts did not reuse the released slave number");
}

pub struct ForkWriter {
    identity: ProcessIdentity,
    signal: File,
    finished: bool,
}

impl ForkWriter {
    pub fn spawn(old: &PtyPair) -> Self {
        let [child_channel, signal] = descriptor::channel();
        let pipe = [child_channel.as_raw_fd(), signal.as_raw_fd()];
        let path = CString::new(old.path.to_str().unwrap()).unwrap();
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork stale PTY writer");
        if pid == 0 {
            child_writer(pipe, old.master.as_raw_fd(), old.slave.as_raw_fd(), &path);
        }
        drop(child_channel);
        let identity = ProcessIdentity::capture(pid as u32).expect("capture stale writer identity");
        let mut writer = Self {
            identity,
            signal,
            finished: false,
        };
        descriptor::wait_ready(&mut writer.signal);
        writer
    }

    pub fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub fn release(&mut self) {
        self.signal
            .write_all(WRITER_SIGNAL)
            .expect("release stale writer");
    }

    pub fn signal_descriptor(&self) -> RawFd {
        self.signal.as_raw_fd()
    }

    pub fn wait(&mut self) {
        if self.finished {
            return;
        }
        let mut status = 0;
        loop {
            let result = unsafe {
                libc::waitpid(
                    self.identity.pid as libc::pid_t,
                    &mut status,
                    WAIT_ANY_OPTIONS,
                )
            };
            if result >= 0 {
                break;
            }
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                ErrorKind::Interrupted
            );
        }
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), CHILD_SUCCESS);
        self.finished = true;
    }
}

impl Drop for ForkWriter {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.identity.signal(libc::SIGKILL);
            let mut status = 0;
            unsafe {
                libc::waitpid(
                    self.identity.pid as libc::pid_t,
                    &mut status,
                    WAIT_ANY_OPTIONS,
                )
            };
        }
    }
}

fn child_writer(pipe: [RawFd; 2], old_master: RawFd, old_slave: RawFd, path: &CString) -> ! {
    unsafe {
        libc::close(pipe[1]);
        libc::close(old_master);
        libc::close(old_slave);
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        if libc::write(
            pipe[0],
            descriptor::WRITER_READY.as_ptr().cast(),
            descriptor::WRITER_READY.len(),
        ) != descriptor::WRITER_READY.len() as isize
        {
            libc::_exit(CHILD_FAILURE);
        }
        let mut signal = 0_u8;
        let ready = libc::read(pipe[0], (&mut signal as *mut u8).cast(), 1) == 1;
        let slave = libc::open(
            path.as_ptr(),
            libc::O_WRONLY | libc::O_NOCTTY | libc::O_CLOEXEC,
        );
        let wrote = slave >= 0
            && libc::write(slave, STALE_PAYLOAD.as_ptr().cast(), STALE_PAYLOAD.len())
                == STALE_PAYLOAD.len() as isize;
        if slave >= 0 {
            libc::close(slave);
        }
        libc::_exit(if ready && wrote {
            CHILD_SUCCESS
        } else {
            CHILD_FAILURE
        });
    }
}
