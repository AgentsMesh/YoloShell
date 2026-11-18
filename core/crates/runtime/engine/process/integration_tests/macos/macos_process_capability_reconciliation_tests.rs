use super::*;
use crate::process::{DetachedStandardError, LiveProcessIdentity, ProcessIdentity, spawn_detached};
use std::path::Path;
use std::sync::Arc;

use super::monitor_test_support::wait_for;

const ROOT_EXECUTABLE: &str = "/bin/sleep";
const ROOT_PARK_DURATION: &str = "30";
const ROOT_GENERATION_INCREMENT: u64 = 1;

struct Fixture {
    child: std::process::Child,
    live: LiveProcessIdentity,
    reader: Arc<MacosProcessCapabilityManagerReader>,
    writer: Option<MacosProcessCapabilityChildWriter>,
}

impl Fixture {
    fn spawn() -> Self {
        let child = spawn_detached(
            Path::new(ROOT_EXECUTABLE),
            &[ROOT_PARK_DURATION.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let live = LiveProcessIdentity::capture(root).unwrap().unwrap();
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (reader, writer) = test_support::transfer(issuer).unwrap();
        reader.arm_liveness(live).unwrap();
        Self {
            child,
            live,
            reader,
            writer: Some(writer),
        }
    }

    fn close_writer(&mut self) {
        drop(self.writer.take());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn reconciliation_observes_writer_close_from_cached_root_exit() {
    let mut fixture = Fixture::spawn();

    fixture.child.kill().unwrap();
    fixture.child.wait().unwrap();
    wait_for_root_exit(&fixture.reader);
    fixture.close_writer();
    fixture.reader.reconcile_armed_root_exit().unwrap();

    assert_eq!(
        fixture.reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn exact_confirmation_reconciles_cached_root_exit_and_writer_close() {
    let mut fixture = Fixture::spawn();
    fixture.reader.confirm_root_exit(&fixture.live).unwrap();
    assert_eq!(
        fixture.reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::RootExited)
    );

    fixture.close_writer();
    fixture.reader.confirm_root_exit(&fixture.live).unwrap();

    assert_eq!(
        fixture.reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn exact_confirmation_settles_pending_release_after_identity_disappears() {
    let mut fixture = Fixture::spawn();
    fixture.close_writer();
    wait_for(
        &fixture.reader,
        MacosProcessCapabilityObservation::ReleasedPendingRootExit,
    );
    fixture.child.kill().unwrap();
    fixture.child.wait().unwrap();

    fixture.reader.confirm_root_exit(&fixture.live).unwrap();

    assert_eq!(
        fixture.reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::ReleasedAfterRootExit)
    );
    assert_eq!(
        fixture.reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn exact_confirmation_rejects_changed_root_before_reconciliation() {
    let fixture = Fixture::spawn();
    let changed = fixture.live.with_generation(
        fixture
            .live
            .unique_id()
            .wrapping_add(ROOT_GENERATION_INCREMENT),
    );

    assert!(fixture.reader.confirm_root_exit(&changed).is_err());
    assert_eq!(
        fixture.reader.liveness_observation(),
        Some(MacosProcessCapabilityObservation::WriterPresent)
    );
}

fn wait_for_root_exit(reader: &MacosProcessCapabilityManagerReader) {
    wait_for(reader, MacosProcessCapabilityObservation::RootExited);
}
