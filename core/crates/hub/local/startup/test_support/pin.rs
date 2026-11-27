use crate::{Launch, constants::*};
use std::{future::Future, path::PathBuf, pin::Pin, task::Poll, time::Duration};
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_state::{HUB_IDENTITY_FILE, HubState};
use yolo_paths::coordination::CoordinationLock;

pub const STATE_NAME: &str = "hub-state";
pub const HOST_NAME: &str = "host-home";
pub const SAVED_IDENTITY: &str = "saved-identity";
pub const SAVED_DIRECTORY: &str = "saved-directory";
const ABSENT_EXECUTABLE: &str = "must-not-start";
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub launch: Launch,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let launch = Launch {
            state: base.join(STATE_NAME),
            host_home: base.join(HOST_NAME),
            host_executable: base.join(ABSENT_EXECUTABLE),
            hub_executable: base.join(ABSENT_EXECUTABLE),
        };
        Self { root, launch }
    }
    pub fn initialize(&self) -> PeerIdentity {
        HubState::open(&self.launch.state, &self.launch.host_home)
            .unwrap()
            .peer_identity()
    }
    pub fn identity_path(&self) -> PathBuf {
        self.launch.state.join(HUB_IDENTITY_FILE)
    }
    pub fn inspect(&self) -> Option<PeerIdentity> {
        HubState::inspect_peer_identity(&self.launch.state, &self.launch.host_home).unwrap()
    }
    pub fn lock(&self) -> CoordinationLock {
        CoordinationLock::try_acquire(&self.launch.state.join(START_LOCK_FILE), MAX_LOCK_BYTES)
            .unwrap()
            .unwrap()
    }
    pub fn displace_identity(&self) {
        std::fs::rename(self.identity_path(), self.root.path().join(SAVED_IDENTITY)).unwrap();
    }
}
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}

/// Polls through lock acquisition into the timer while the competing lock is held.
pub async fn assert_waiting<F: Future>(mut future: Pin<&mut F>) {
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
}
