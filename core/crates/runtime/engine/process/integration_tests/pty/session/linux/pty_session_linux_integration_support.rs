use super::super::inventory_snapshot::install_test_inventory;
use super::super::inventory_stable::StableInventory;
use super::super::inventory_test_support::{NO_PARENT_PID, entry};
use super::super::pty_session_proof::{
    CapabilityScanStatus, PtyCapabilityScanState, PtySessionProof,
};
use super::super::{PTY_SESSION_CAPABILITY_ENV, ProcessIdentity, procfs};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::ptr;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_SECONDS: &str = "30";
const SYSTEM_CALL_FAILURE: libc::c_int = -1;
const ROOT_EXITED: bool = false;
const SCAN_ESCAPED_PROCESSES: bool = false;
const CANDIDATE_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const CANDIDATE_STOP_POLL: Duration = Duration::from_millis(10);
const CANDIDATE_STOP_DETAIL: &str = "PTY descriptor candidate did not reach its stopped barrier";

pub(super) struct TestPty {
    _master: OwnedFd,
    slave: Option<OwnedFd>,
    pub(super) proof: PtySessionProof,
}

impl TestPty {
    pub(super) fn open(capability: &str) -> Self {
        let mut master = SYSTEM_CALL_FAILURE;
        let mut slave = SYSTEM_CALL_FAILURE;
        // SAFETY: openpty initializes both descriptors and retains no pointers.
        let result = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        assert_eq!(result, libc::EXIT_SUCCESS);
        // SAFETY: successful openpty returned two newly owned descriptors.
        let (master, slave) =
            unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
        set_close_on_exec(master.as_raw_fd(), true);
        set_close_on_exec(slave.as_raw_fd(), false);
        let descriptor =
            procfs::file_descriptors_path(std::process::id()).join(slave.as_raw_fd().to_string());
        let tty_path = std::fs::read_link(descriptor).unwrap();
        let proof = PtySessionProof::capture(tty_path, capability.to_owned()).unwrap();
        Self {
            _master: master,
            slave: Some(slave),
            proof,
        }
    }

    pub(super) fn slave_fd(&self) -> libc::c_int {
        self.slave.as_ref().unwrap().as_raw_fd()
    }

    pub(super) fn close_slave(&mut self) {
        self.slave.take();
    }
}

pub(super) struct CandidateFixture {
    child: Option<Child>,
    pub(super) identity: ProcessIdentity,
}

impl CandidateFixture {
    pub(super) fn spawn(capability: Option<&str>) -> Self {
        let mut command = Command::new(SLEEP_EXECUTABLE);
        command.arg(SLEEP_DURATION_SECONDS);
        match capability {
            Some(capability) => command.env(PTY_SESSION_CAPABILITY_ENV, capability),
            None => command.env_remove(PTY_SESSION_CAPABILITY_ENV),
        };
        configure_session(&mut command);
        let child = spawn_session(command);
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let candidate = Self {
            child: Some(child),
            identity,
        };
        candidate.stop_descriptor_changes();
        candidate
    }

    fn stop_descriptor_changes(&self) {
        // exec completion precedes dynamic-loader/locale initialization. Stop the
        // exact fixture child before asserting one complete descriptor snapshot.
        assert!(self.identity.signal(libc::SIGSTOP).unwrap());
        let live = super::super::LiveProcessIdentity::capture(self.identity)
            .unwrap()
            .unwrap();
        let deadline = Instant::now() + CANDIDATE_STOP_TIMEOUT;
        while live.execution_state().unwrap()
            != crate::process::live_identity::ProcessExecutionState::Stopped
        {
            assert!(
                live.try_matches().unwrap() && Instant::now() < deadline,
                "{CANDIDATE_STOP_DETAIL}"
            );
            std::thread::sleep(CANDIDATE_STOP_POLL);
        }
    }
}

impl Drop for CandidateFixture {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

pub(super) fn candidate_matches(proof: &PtySessionProof, identity: ProcessIdentity) -> bool {
    let candidate = entry(identity, NO_PARENT_PID, identity.pid);
    let _inventory = install_test_inventory([vec![candidate], vec![candidate]]);
    let inventory = StableInventory::capture().unwrap();
    let state = PtyCapabilityScanState::default();
    let result = inventory
        .capability_candidates(
            Some(proof),
            &state,
            &Default::default(),
            &Default::default(),
            identity,
            u64::default(),
            ROOT_EXITED,
            SCAN_ESCAPED_PROCESSES,
        )
        .unwrap();
    assert_eq!(result.candidates.status, CapabilityScanStatus::Complete);
    result
        .candidates
        .identities
        .iter()
        .any(|candidate| candidate.durable == identity)
}

pub(super) fn configure_session(command: &mut Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: the closure calls only setsid before exec and reports errno directly.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == SYSTEM_CALL_FAILURE {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

pub(super) fn spawn_session(command: Command) -> Child {
    crate::process::spawn_waitable_test_child(command)
}

fn set_close_on_exec(descriptor: libc::c_int, enabled: bool) {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, SYSTEM_CALL_FAILURE);
    let flags = if enabled {
        flags | libc::FD_CLOEXEC
    } else {
        flags & !libc::FD_CLOEXEC
    };
    assert_ne!(
        unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags) },
        SYSTEM_CALL_FAILURE
    );
}
