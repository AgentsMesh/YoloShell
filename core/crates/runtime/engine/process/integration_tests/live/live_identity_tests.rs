use super::captured_targets;
use super::{LiveProcessIdentity, ProcessIdentity};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const NEXT_GENERATION: u64 = 1;
const FIRST_GENERATION: u64 = 1;

struct ChildGuard {
    child: Option<std::process::Child>,
    durable: ProcessIdentity,
    live: LiveProcessIdentity,
}

impl ChildGuard {
    fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            std::process::Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let durable = ProcessIdentity::capture(child.id()).unwrap();
        let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
        Self {
            child: Some(child),
            durable,
            live,
        }
    }

    fn reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.wait().unwrap();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.live.signal(libc::SIGKILL);
        self.reap();
    }
}

#[test]
fn forged_unique_id_cannot_signal_a_matching_numeric_pid() {
    let mut child = ChildGuard::spawn();
    let forged = forged(&child.live);

    assert!(!forged.signal(libc::SIGKILL).unwrap());
    assert!(child.durable.matches());
    assert!(child.live.signal(libc::SIGKILL).unwrap());
    child.reap();
}

#[test]
fn captured_targets_never_recanonicalize_a_durable_projection() {
    let mut child = ChildGuard::spawn();
    let forged = forged(&child.live);

    captured_targets::signal_for_test(&[(forged, child.durable)], libc::SIGKILL).unwrap();

    assert!(child.durable.matches());
    assert!(child.live.signal(libc::SIGKILL).unwrap());
    child.reap();
}

fn forged(identity: &LiveProcessIdentity) -> LiveProcessIdentity {
    let generation = identity.lineage_id().wrapping_add(NEXT_GENERATION);
    let generation = (generation > 0)
        .then_some(generation)
        .unwrap_or(FIRST_GENERATION);
    identity.with_generation(generation)
}
