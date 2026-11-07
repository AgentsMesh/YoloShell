use super::*;

const PRECOMMIT_OUTPUT: &[u8] = b"survives-precommit-namespace-loss";
const MOVED_SESSION_DIRECTORY: &str = "moved-session";
const ADVERSARIAL_RETAINED_FAILURES: usize = 4;
const SINGLE_RETAINED_FAILURE: usize = 1;

struct NamespaceReplacingReader {
    session_dir: std::path::PathBuf,
    moved_dir: std::path::PathBuf,
    replaced: bool,
}

impl Read for NamespaceReplacingReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.replaced {
            return Ok(usize::default());
        }
        std::fs::rename(&self.session_dir, &self.moved_dir)?;
        self.replaced = true;
        buffer[..PRECOMMIT_OUTPUT.len()].copy_from_slice(PRECOMMIT_OUTPUT);
        Ok(PRECOMMIT_OUTPUT.len())
    }
}

fn displace_session_and_retain_journal(fixture: &Fixture) -> std::path::PathBuf {
    let session = fixture.state.paths.session_dir(fixture.terminal_id());
    let journal_suffix = fixture
        .state
        .paths
        .journal(fixture.terminal_id())
        .strip_prefix(&session)
        .unwrap()
        .to_path_buf();
    let displaced = fixture._filesystem.scratch_path(MOVED_SESSION_DIRECTORY);
    std::fs::rename(session, &displaced).unwrap();
    displaced.join(journal_suffix)
}

#[test]
fn namespace_loss_before_precommit_drains_until_real_kernel_eof() {
    let fixture = Fixture::new();
    let session_dir = fixture.state.paths.session_dir(fixture.terminal_id());
    let moved_dir = fixture._filesystem.scratch_path(MOVED_SESSION_DIRECTORY);
    std::fs::rename(session_dir, moved_dir).unwrap();
    let reads = Arc::new(std::sync::atomic::AtomicUsize::default());
    let reader = CountingReader {
        reads: Arc::clone(&reads),
    };
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();

    assert_eq!(
        reader_loop(Box::new(reader), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::ChildExited
    );
    assert_eq!(reads.load(Ordering::Acquire), OUTPUT_EXPECTED_READS);
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(
        fixture
            .state
            .reader_lifecycle
            .kernel_eof_identity()
            .is_some()
    );
    assert_eq!(
        fixture.state.reader_lifecycle.output_progress_state(),
        OutputProgressState::Abandoned
    );
    assert_eq!(
        fixture.state.journal.lock().unwrap().last_sequence(),
        u64::default()
    );
}

#[test]
fn namespace_loss_during_kernel_read_keeps_output_recoverable() {
    let fixture = Fixture::new();
    let session_dir = fixture.state.paths.session_dir(fixture.terminal_id());
    let moved_dir = fixture._filesystem.scratch_path(MOVED_SESSION_DIRECTORY);
    let journal_suffix = fixture
        .state
        .paths
        .journal(fixture.terminal_id())
        .strip_prefix(&session_dir)
        .unwrap()
        .to_path_buf();
    let reader = NamespaceReplacingReader {
        session_dir,
        moved_dir: moved_dir.clone(),
        replaced: false,
    };
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture
        .state
        .reader_lifecycle
        .mark_producers_gone_for_test();

    assert_eq!(
        reader_loop(Box::new(reader), Arc::clone(&fixture.state)).unwrap(),
        ReaderLoopOutcome::ChildExited
    );
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(
        fixture
            .state
            .reader_lifecycle
            .kernel_eof_identity()
            .is_some()
    );
    let mut recovered = Journal::open(
        moved_dir.join(journal_suffix),
        fixture.state.runtime_epoch(),
    )
    .unwrap();
    let chunks = recovered.replay_after(u64::default(), usize::MAX).unwrap();
    assert_eq!(chunks.len(), OUTPUT_TEST_SEQUENCE as usize);
    assert_eq!(chunks[usize::default()].data, PRECOMMIT_OUTPUT);
}

#[test]
fn retained_commit_retries_repeated_transient_failures_without_output_loss() {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    stage_reader_output(&fixture.state, PRECOMMIT_OUTPUT).unwrap();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .inject_retained_reconcile_failures(ADVERSARIAL_RETAINED_FAILURES);
    let moved_journal = displace_session_and_retain_journal(&fixture);

    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::NamespaceLost
    );
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert!(!fixture.state.reader_lifecycle.drains_after_namespace_loss());
    assert_eq!(fixture.state.termination_reason(), None);
    commit_tests::assert_confirmed_exactly_once(&fixture, moved_journal, &mut events);
}

#[test]
fn retained_commit_recovers_after_single_retry_exactly_once() {
    let fixture = Fixture::new();
    let mut events = fixture.state.events.subscribe();
    stage_reader_output(&fixture.state, PRECOMMIT_OUTPUT).unwrap();
    fixture
        .state
        .journal
        .lock()
        .unwrap()
        .inject_retained_reconcile_failures(SINGLE_RETAINED_FAILURE);
    let moved_journal = displace_session_and_retain_journal(&fixture);

    assert_eq!(
        deliver_pending_with_retry(&fixture.state).unwrap(),
        PendingDelivery::NamespaceLost
    );
    assert!(!fixture.state.reader_lifecycle.has_pending());
    assert!(!fixture.state.reader_lifecycle.drains_after_namespace_loss());
    assert_eq!(fixture.state.termination_reason(), None);
    commit_tests::assert_confirmed_exactly_once(&fixture, moved_journal, &mut events);
}

#[path = "output_namespace_loss_commit_tests.rs"]
mod commit_tests;
use crate::worker::pty_output_progress::OutputProgressState;
