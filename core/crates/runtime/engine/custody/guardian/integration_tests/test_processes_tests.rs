use super::*;
use std::process::{Command, Stdio};

const TEST_PROCESS: &str = "/bin/sleep";
const TEST_PROCESS_DURATION: &str = "30";

struct Fixture {
    home: tempfile::TempDir,
    identity: ProcessIdentity,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let terminal_id = Uuid::new_v4();
        let settlement = crate::process::reserve_detached_child_settlement().unwrap();
        let mut command = Command::new(TEST_PROCESS);
        command
            .arg(TEST_PROCESS_DURATION)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::process::configure_detached_command(&mut command);
        let child = crate::process::spawn_waitable_test_child(command);
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        settlement.retain(child);
        register(
            home.path(),
            terminal_id,
            CustodyHolderIdentity {
                pid: identity.pid,
                start_time: identity.start_time,
            },
        )
        .unwrap();
        Self { home, identity }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        terminate_for_home(self.home.path()).unwrap();
    }
}

#[test]
fn cleanup_is_scoped_to_the_canonical_home() {
    let fixture_a = Fixture::new();
    let fixture_b = Fixture::new();
    terminate_for_home(fixture_a.home.path()).unwrap();
    assert_eq!(
        fixture_a.identity.observe(),
        ProcessIdentityObservation::Gone
    );
    assert_eq!(
        fixture_b.identity.observe(),
        ProcessIdentityObservation::Matching
    );
}
