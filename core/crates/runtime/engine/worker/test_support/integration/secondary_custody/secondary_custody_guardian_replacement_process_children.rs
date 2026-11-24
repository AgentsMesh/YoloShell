use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::{CustodyGuardianPreparation, CustodyHolderIdentity};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION_ARGUMENT: &str = "30";
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(8);
const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PROCESS_EXIT_TIMEOUT_DETAIL: &str = "replacement process cleanup timed out";
const PROCESS_NOT_TERMINATED_DETAIL: &str = "replacement process was not force-terminated";
const GUARDIAN_CLEANUP_DETAIL: &str = "clean up replacement test custody guardian processes";

pub(super) struct ExactChild {
    child: Option<Child>,
    identity: ProcessIdentity,
}

impl ExactChild {
    pub(super) fn spawn(
        writer: Option<&crate::process::MacosProcessCapabilityChildWriter>,
    ) -> Self {
        let mut command = Command::new(CHILD_EXECUTABLE);
        command
            .arg(CHILD_DURATION_ARGUMENT)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(writer) = writer {
            writer.configure_exec_inheritance_for_test(&mut command);
        }
        let child = crate::process::spawn_waitable_test_child(command);
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child: Some(child),
            identity,
        }
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    pub(super) fn terminate(&mut self) {
        let child = self.child.as_mut().expect(PROCESS_NOT_TERMINATED_DETAIL);
        assert!(self.identity.signal(libc::SIGKILL).unwrap());
        let status = child.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        self.child = None;
        assert_gone(self.identity);
    }

    pub(super) fn wait_for_guardian_termination(&mut self) {
        let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
        let status = loop {
            let child = self.child.as_mut().expect(PROCESS_NOT_TERMINATED_DETAIL);
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "{PROCESS_EXIT_TIMEOUT_DETAIL}");
            std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
        };
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        self.child = None;
        assert_gone(self.identity);
    }
}

impl Drop for ExactChild {
    fn drop(&mut self) {
        let Some(child) = self.child.as_mut() else {
            return;
        };
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = child.wait();
        self.child = None;
    }
}

pub(crate) struct DetachedGuardian {
    identity: Option<ProcessIdentity>,
}

impl DetachedGuardian {
    pub(crate) fn capture(preparation: CustodyGuardianPreparation) -> Self {
        let process = preparation.binding().process();
        Self {
            identity: Some(ProcessIdentity {
                pid: process.pid,
                start_time: process.start_time,
            }),
        }
    }

    pub(crate) fn wait_for_natural_exit(&mut self) {
        let identity = self.identity.expect(PROCESS_NOT_TERMINATED_DETAIL);
        wait_until_gone(identity);
        self.identity = None;
    }

    pub(crate) fn assert_current(&self) {
        let identity = self.identity.expect(PROCESS_NOT_TERMINATED_DETAIL);
        assert_eq!(identity.observe(), ProcessIdentityObservation::Matching);
    }

    pub(crate) fn terminate(&mut self) {
        let identity = self.identity.expect(PROCESS_NOT_TERMINATED_DETAIL);
        assert!(identity.signal(libc::SIGKILL).unwrap());
        wait_until_gone(identity);
        self.identity = None;
    }
}

impl Drop for DetachedGuardian {
    fn drop(&mut self) {
        let Some(identity) = self.identity.take() else {
            return;
        };
        let _ = identity.signal(libc::SIGKILL);
        let _ = wait_until_gone_before(identity, Instant::now() + PROCESS_EXIT_TIMEOUT);
    }
}

impl Drop for super::Fixture {
    fn drop(&mut self) {
        if let Err(error) =
            crate::custody_guardian::terminate_test_processes_for_home(self.paths.home())
        {
            if std::thread::panicking() {
                eprintln!("{GUARDIAN_CLEANUP_DETAIL}: {error:#}");
            } else {
                panic!("{GUARDIAN_CLEANUP_DETAIL}: {error:#}");
            }
        }
    }
}

pub(super) fn gone_holder() -> CustodyHolderIdentity {
    let mut witness = ExactChild::spawn(None);
    let identity = witness.identity();
    witness.terminate();
    CustodyHolderIdentity {
        pid: identity.pid,
        start_time: identity.start_time,
    }
}

fn assert_gone(identity: ProcessIdentity) {
    assert_eq!(identity.observe(), ProcessIdentityObservation::Gone);
}

fn wait_until_gone(identity: ProcessIdentity) {
    assert!(wait_until_gone_before(
        identity,
        Instant::now() + PROCESS_EXIT_TIMEOUT
    ));
}

fn wait_until_gone_before(identity: ProcessIdentity, deadline: Instant) -> bool {
    while identity.observe() != ProcessIdentityObservation::Gone {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
    true
}
