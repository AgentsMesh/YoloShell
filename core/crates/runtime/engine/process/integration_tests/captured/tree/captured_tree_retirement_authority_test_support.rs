use crate::process::captured_tree::gone;
use crate::process::{
    CapturedProcessTree, ProcessIdentity, PtyMasterAliasReleaseAuthority, PtySessionProof,
};
use std::process::{Child, Command};
use uuid::Uuid;

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
#[cfg(not(target_os = "macos"))]
const TEST_TTY_PATH: &str = "/dev/null";
pub(super) const TEST_CAPABILITY: &str = "retirement-authority-capability";
#[cfg(target_os = "macos")]
const TEST_ROWS: u16 = 24;
#[cfg(target_os = "macos")]
const TEST_COLUMNS: u16 = 80;
#[cfg(target_os = "macos")]
const TEST_PIXEL_SIZE: u16 = 0;

pub(super) struct ChildGuard {
    child: Child,
    pub(super) identity: ProcessIdentity,
}

impl ChildGuard {
    pub(super) fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self { child, identity }
    }
}

pub(super) fn pending(
    root: ProcessIdentity,
    session: Uuid,
    epoch: Uuid,
) -> (
    crate::process::MasterReleasePendingProducersGone,
    PtyMasterAliasReleaseAuthority,
) {
    let mut proof = pending_proof();
    proof.bind_runtime(session, epoch).unwrap();
    let release = proof.bind_master_alias_release(root).unwrap();
    #[cfg(target_os = "macos")]
    proof.release_local_object_lease().unwrap();
    let tree = CapturedProcessTree::capture_pty(root, proof).unwrap();
    let tracked = gone::complete(tree);
    let (reported, ()) = tracked.report(|_| Ok(())).unwrap();
    (reported.into_master_release_for_test(), release)
}

#[cfg(target_os = "macos")]
fn pending_proof() -> PtySessionProof {
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap()
}

#[cfg(not(target_os = "macos"))]
fn pending_proof() -> PtySessionProof {
    PtySessionProof::capture(TEST_TTY_PATH.into(), TEST_CAPABILITY.to_owned()).unwrap()
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}
