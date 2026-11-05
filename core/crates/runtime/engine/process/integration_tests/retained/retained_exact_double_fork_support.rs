use super::{ProcessIdentity, PtySessionProof};
use portable_pty::{PtySize, native_pty_system};
use std::path::{Path, PathBuf};

#[path = "../../test_support/retained/retained_exact_double_fork_helpers.rs"]
mod helpers;
const TRIGGER_FILE: &str = "trigger";
const READY_FILE: &str = "leaf.ready";
const PID_FILE: &str = "leaf.pid";
const EXIT_FILE: &str = "root.exit";
const EMPTY: &[u8] = b"";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const PIXEL_DIMENSION: u16 = 0;
const READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const READY_POLL: std::time::Duration = std::time::Duration::from_millis(10);
const ROOT_EXIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const ROOT_EXIT_POLL: std::time::Duration = std::time::Duration::from_millis(10);
const ROOT_EXIT_TIMEOUT_DETAIL: &str = "escaped-descendant root exit timed out";

#[derive(Clone, Copy)]
enum FixtureProofAuthority {
    ObservationOnly,
    RetainedBootstrapMaster,
}

pub(super) struct DoubleForkFixture {
    _directory: tempfile::TempDir,
    _master: Box<dyn portable_pty::MasterPty + Send>,
    root: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    pub(super) root_identity: ProcessIdentity,
    pub(super) proof: PtySessionProof,
    pub(super) trigger: PathBuf,
    ready: PathBuf,
    leaf_pid: PathBuf,
    leaf: Option<ProcessIdentity>,
}

impl DoubleForkFixture {
    pub(super) fn spawn(cloak: bool) -> Self {
        Self::spawn_with_authority(cloak, FixtureProofAuthority::ObservationOnly)
    }

    pub(super) fn spawn_with_retained_bootstrap_master(cloak: bool) -> Self {
        Self::spawn_with_authority(cloak, FixtureProofAuthority::RetainedBootstrapMaster)
    }

    fn spawn_with_authority(cloak: bool, authority: FixtureProofAuthority) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let trigger = directory.path().join(TRIGGER_FILE);
        let ready = directory.path().join(READY_FILE);
        let leaf_pid = directory.path().join(PID_FILE);
        let exit = directory.path().join(EXIT_FILE);
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: TERMINAL_ROWS,
                cols: TERMINAL_COLUMNS,
                pixel_width: PIXEL_DIMENSION,
                pixel_height: PIXEL_DIMENSION,
            })
            .unwrap();
        let capability = uuid::Uuid::new_v4().to_string();
        let proof = match authority {
            FixtureProofAuthority::ObservationOnly => {
                PtySessionProof::capture(pair.master.tty_name().unwrap(), capability.clone())
            }
            FixtureProofAuthority::RetainedBootstrapMaster => {
                PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), capability.clone())
            }
        }
        .unwrap();
        let mut command = helpers::root_command(&trigger, &ready, &leaf_pid, &exit, cloak);
        command.env(super::constants::PTY_SESSION_CAPABILITY_ENV, capability);
        let _spawn_guard = super::waitable_child_spawn_guard().unwrap();
        let root = pair.slave.spawn_command(command).unwrap();
        let root_pid = root.process_id().unwrap();
        let root_identity = ProcessIdentity::capture(root_pid).unwrap();
        drop(pair.slave);
        Self {
            _directory: directory,
            _master: pair.master,
            root: Some(root),
            root_identity,
            proof,
            trigger,
            ready,
            leaf_pid,
            leaf: None,
        }
    }

    pub(super) fn release_leaf(&mut self) -> ProcessIdentity {
        std::fs::write(&self.trigger, EMPTY).unwrap();
        wait_for(&self.ready);
        let pid = std::fs::read_to_string(&self.leaf_pid)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let leaf = ProcessIdentity::capture(pid).unwrap();
        self.leaf = Some(leaf);
        leaf
    }

    pub(super) fn terminate_root(&mut self) {
        assert!(self.root_identity.signal(libc::SIGKILL).unwrap());
        assert!(
            self.reap_root(ROOT_EXIT_TIMEOUT),
            "{ROOT_EXIT_TIMEOUT_DETAIL}"
        );
    }

    fn reap_root(&mut self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while let Some(root) = self.root.as_mut() {
            match root.try_wait() {
                Ok(Some(_)) => {
                    self.root.take();
                    return true;
                }
                Err(_) => return false,
                Ok(None) => {}
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(ROOT_EXIT_POLL);
        }
        true
    }
}

impl Drop for DoubleForkFixture {
    fn drop(&mut self) {
        if let Some(leaf) = self.leaf {
            let _ = leaf.signal(libc::SIGKILL);
        }
        if self.root.is_some() {
            let _ = self.root_identity.signal(libc::SIGKILL);
            let _ = self.reap_root(ROOT_EXIT_TIMEOUT);
        }
    }
}

#[test]
fn double_fork_helper() {
    helpers::run_role();
}

fn wait_for(path: &Path) {
    let deadline = std::time::Instant::now() + READY_TIMEOUT;
    while !path.exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(READY_POLL);
    }
}
