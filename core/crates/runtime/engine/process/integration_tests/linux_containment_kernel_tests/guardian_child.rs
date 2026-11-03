use super::support::KERNEL_SETTLE_TIMEOUT;
use crate::process::ProcessIdentity;

const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const WAIT_OPTIONS: libc::c_int = 0;
const POLL_DESCRIPTOR_COUNT: libc::nfds_t = 1;

pub struct ChildProcess {
    identity: ProcessIdentity,
    finished: bool,
}

impl ChildProcess {
    pub fn new(pid: libc::pid_t) -> Self {
        Self {
            identity: ProcessIdentity::capture(pid as u32).expect("capture fixture child identity"),
            finished: false,
        }
    }

    pub fn kill(&mut self) {
        if self.finished {
            return;
        }
        assert!(
            self.identity
                .signal(libc::SIGKILL)
                .expect("signal exact fixture child")
        );
        let status = self.wait();
        assert!(libc::WIFSIGNALED(status));
        assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
    }

    pub fn wait_for_exit(&mut self) {
        let status = self.wait();
        assert!(libc::WIFEXITED(status));
        assert_eq!(libc::WEXITSTATUS(status), SYSTEM_CALL_SUCCESS);
    }

    fn wait(&mut self) -> libc::c_int {
        let mut status = libc::c_int::default();
        loop {
            let pid = self.identity.pid as libc::pid_t;
            let result = unsafe { libc::waitpid(pid, &mut status, WAIT_OPTIONS) };
            if result == pid {
                self.finished = true;
                return status;
            }
            assert_eq!(
                std::io::Error::last_os_error().kind(),
                std::io::ErrorKind::Interrupted
            );
        }
    }
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.identity.signal(libc::SIGKILL);
            let mut status = libc::c_int::default();
            let _ = unsafe {
                libc::waitpid(self.identity.pid as libc::pid_t, &mut status, WAIT_OPTIONS)
            };
        }
    }
}

pub fn wait_readable(descriptor: libc::c_int) {
    let mut poll = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: libc::c_short::default(),
    };
    let timeout = libc::c_int::try_from(KERNEL_SETTLE_TIMEOUT.as_millis())
        .expect("kernel test timeout fits poll");
    assert_eq!(
        unsafe { libc::poll(&mut poll, POLL_DESCRIPTOR_COUNT, timeout) },
        POLL_DESCRIPTOR_COUNT as libc::c_int
    );
    assert_ne!(poll.revents & libc::POLLIN, 0);
}
