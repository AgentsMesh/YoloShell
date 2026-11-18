use super::*;
use std::collections::BTreeMap;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";
const START_TIME_MISMATCH_INCREMENT: u64 = 1;
const CANDIDATE_PID: u32 = 41_001;
const CANDIDATE_START_TIME: u64 = 51_001;
const CANDIDATE_PARENT_PID: u32 = 61_001;
const CANDIDATE_PROCESS_GROUP: i32 = 71_001;
const CANDIDATE_EXECUTABLE: &str = "fenced-process-candidate";

struct StubProcessSource {
    calls: AtomicUsize,
    candidates: Vec<ObservedProcess>,
    root_to_exit: Mutex<Option<Child>>,
}

impl StubProcessSource {
    fn stable(candidates: Vec<ObservedProcess>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            candidates,
            root_to_exit: Mutex::new(None),
        }
    }

    fn exiting(root: Child, candidates: Vec<ObservedProcess>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            candidates,
            root_to_exit: Mutex::new(Some(root)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Acquire)
    }
}

impl ProcessSource for StubProcessSource {
    fn descendants(&self, _: u32) -> Result<Vec<ObservedProcess>> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        if let Some(mut root) = self.root_to_exit.lock().unwrap().take() {
            root.kill().unwrap();
            root.wait().unwrap();
        }
        Ok(self.candidates.clone())
    }
}

impl Drop for StubProcessSource {
    fn drop(&mut self) {
        if let Some(mut root) = self.root_to_exit.get_mut().unwrap().take() {
            let _ = root.kill();
            let _ = root.wait();
        }
    }
}

fn candidate() -> ObservedProcess {
    ObservedProcess {
        pid: CANDIDATE_PID,
        start_time: CANDIDATE_START_TIME,
        parent_pid: CANDIDATE_PARENT_PID,
        process_group: CANDIDATE_PROCESS_GROUP,
        foreground_group: CANDIDATE_PROCESS_GROUP,
        executable: CANDIDATE_EXECUTABLE.to_owned(),
        argv: vec![CANDIDATE_EXECUTABLE.to_owned()],
        environment: Some(BTreeMap::new()),
        cwd: None,
        evidence_paths: None,
    }
}

#[test]
fn system_process_source_observes_without_external_inventory() {
    let source = RuntimeProcessSource::new(std::iter::empty());
    let descendants = source.descendants(std::process::id()).unwrap();
    assert!(descendants.iter().all(|process| process.pid > 0));
}

#[test]
fn fenced_descendants_skips_scan_when_root_identity_mismatches() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let mismatched = ProcessIdentity {
        pid: root.pid,
        start_time: root
            .start_time
            .saturating_add(START_TIME_MISMATCH_INCREMENT),
    };
    let source = StubProcessSource::stable(vec![candidate()]);
    let object: &dyn ProcessSource = &source;

    assert_eq!(
        object.fenced_descendants(mismatched).unwrap(),
        FencedDescendants::RootChanged
    );
    assert_eq!(source.calls(), 0);
}

#[test]
fn fenced_descendants_rejects_candidates_when_root_exits_during_scan() {
    let mut command = Command::new(TEST_SLEEP_EXECUTABLE);
    command.arg(TEST_SLEEP_DURATION);
    let root = crate::process::spawn_waitable_test_child(command);
    let identity = ProcessIdentity::capture(root.id()).unwrap();
    let source = StubProcessSource::exiting(root, vec![candidate()]);
    let object: &dyn ProcessSource = &source;

    assert_eq!(
        object.fenced_descendants(identity).unwrap(),
        FencedDescendants::RootChanged
    );
    assert_eq!(source.calls(), 1);
}

#[test]
fn fenced_descendants_returns_candidates_when_root_remains_stable() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let expected = vec![candidate()];
    let source = StubProcessSource::stable(expected.clone());
    let object: &dyn ProcessSource = &source;

    assert_eq!(
        object.fenced_descendants(root).unwrap(),
        FencedDescendants::Stable(expected)
    );
    assert_eq!(source.calls(), 1);
}

#[test]
fn callback_lineage_default_preserves_mock_descendants_and_root_identity_fences() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let expected = vec![candidate()];
    let source = StubProcessSource::stable(expected.clone());
    let object: &dyn ProcessSource = &source;
    assert_eq!(
        object.fenced_callback_lineage(root, CANDIDATE_PID).unwrap(),
        FencedDescendants::Stable(expected)
    );
    let mismatched = ProcessIdentity {
        pid: root.pid,
        start_time: root
            .start_time
            .saturating_add(START_TIME_MISMATCH_INCREMENT),
    };
    assert_eq!(
        object
            .fenced_callback_lineage(mismatched, CANDIDATE_PID)
            .unwrap(),
        FencedDescendants::RootChanged
    );
    assert_eq!(source.calls(), 1);
}
