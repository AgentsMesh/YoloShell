use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

const TEST_EXEC_PATH: &[u8] = b"/bin/true\0";
const TEST_RELEASE_BYTE: u8 = b'x';
const TEST_WAIT_TIMEOUT: Duration = Duration::from_secs(2);
const TEST_MISSING_PID: u32 = i32::MAX as u32;
const CROSS_THREAD_CHANNEL_CAPACITY: usize = 1;

#[test]
fn traceexec_accepts_exec_and_detaches_before_immediate_exit() {
    let mut child = ForkedExec::spawn();
    let observer = observer(&child);
    child.release();
    observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap();
    assert!(libc::WIFEXITED(child.wait()));
}

#[test]
fn traceexec_accepts_exec_when_tracee_was_spawned_by_another_thread() {
    let mut child = spawn_from_another_thread();
    let observer = observer(&child);
    child.release();

    observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap();
    assert!(libc::WIFEXITED(child.wait()));
}

#[test]
fn exit_before_exec_is_rejected() {
    let mut child = ForkedExec::spawn();
    let observer = observer(&child);
    child.signal(libc::SIGKILL);
    let error = observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT)
    );
    assert!(libc::WIFSIGNALED(child.wait()));
}

#[test]
fn external_sigtrap_is_not_an_exec_event() {
    let mut child = ForkedExec::spawn();
    let observer = observer(&child);
    child.signal(libc::SIGTRAP);
    let error = observer
        .wait_until(Instant::now() + TEST_WAIT_TIMEOUT)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT)
    );
    child.reap_if_available();
}

#[test]
fn timeout_detaches_and_resumes_the_tracee() {
    let mut child = ForkedExec::spawn();
    let observer = observer(&child);
    let error = observer.wait_until(Instant::now()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT)
    );
    child.release();
    assert!(libc::WIFEXITED(child.wait()));
}

#[test]
fn registration_fails_for_a_missing_thread() {
    let target = BootstrapExecTarget {
        process: ProcessIdentity::capture(std::process::id()).unwrap(),
        thread_id: TEST_MISSING_PID,
    };
    let error = ExecObserver::register(target).err().unwrap();
    assert!(error.to_string().contains(BOOTSTRAP_EXEC_THREAD_CONTEXT));
}

fn observer(child: &ForkedExec) -> ExecObserver {
    let process = ProcessIdentity::capture(child.pid).unwrap();
    ExecObserver::register(BootstrapExecTarget {
        process,
        thread_id: child.pid,
    })
    .unwrap()
}

fn spawn_from_another_thread() -> ForkedExec {
    let (sender, receiver) = std::sync::mpsc::sync_channel(CROSS_THREAD_CHANNEL_CAPACITY);
    let thread = std::thread::spawn(move || sender.send(ForkedExec::spawn()).unwrap());
    let child = receiver.recv().unwrap();
    thread.join().unwrap();
    child
}

struct ForkedExec {
    pid: u32,
    release: Option<OwnedFd>,
    reaped: bool,
}

impl ForkedExec {
    fn spawn() -> Self {
        let mut descriptors = [0; 2];
        // SAFETY: descriptors points to storage for exactly two file descriptors.
        assert_eq!(
            unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) },
            0
        );
        // SAFETY: fork duplicates the process; the child calls only async-signal-safe APIs.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0);
        if pid == 0 {
            child_exec(descriptors);
        }
        // SAFETY: each successful pipe descriptor is transferred exactly once.
        let release = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
        // SAFETY: the parent no longer needs the read side.
        assert_eq!(unsafe { libc::close(descriptors[0]) }, 0);
        Self {
            pid: pid as u32,
            release: Some(release),
            reaped: false,
        }
    }

    fn release(&mut self) {
        let release = self.release.take().unwrap();
        let byte = TEST_RELEASE_BYTE;
        // SAFETY: the pointer references one initialized byte for the duration of the call.
        assert_eq!(
            unsafe {
                libc::write(
                    release.as_raw_fd(),
                    std::ptr::addr_of!(byte).cast(),
                    std::mem::size_of::<u8>(),
                )
            },
            1
        );
    }

    fn signal(&self, signal: libc::c_int) {
        assert_eq!(unsafe { libc::kill(self.pid as libc::pid_t, signal) }, 0);
    }

    fn wait(&mut self) -> libc::c_int {
        let mut status = 0;
        let waited = unsafe { libc::waitpid(self.pid as libc::pid_t, &mut status, 0) };
        assert_eq!(waited, self.pid as libc::pid_t);
        self.reaped = true;
        status
    }

    fn reap_if_available(&mut self) {
        let mut status = 0;
        let waited = unsafe { libc::waitpid(self.pid as libc::pid_t, &mut status, libc::WNOHANG) };
        self.reaped = waited == self.pid as libc::pid_t || waited == -1;
    }
}

impl Drop for ForkedExec {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = unsafe { libc::kill(self.pid as libc::pid_t, libc::SIGKILL) };
            let _ = self.wait();
        }
    }
}

fn child_exec(descriptors: [libc::c_int; 2]) -> ! {
    let mut byte = 0_u8;
    let read = unsafe {
        libc::read(
            descriptors[0],
            std::ptr::addr_of_mut!(byte).cast(),
            std::mem::size_of::<u8>(),
        )
    };
    if read == 1 {
        let executable = TEST_EXEC_PATH.as_ptr().cast::<libc::c_char>();
        unsafe { libc::execl(executable, executable, std::ptr::null::<libc::c_char>()) };
    }
    unsafe { libc::_exit(libc::EXIT_FAILURE) }
}
