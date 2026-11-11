use super::{
    CapturedProcessTree, ExplicitBootstrapExecFailureProof, ProcessIdentity,
    PtyMasterAliasesClosed, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::process::{Child, Command};
use uuid::Uuid;

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const TEST_CAPABILITY: &str = "bootstrap-failure-retirement-capability";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

struct Fixture {
    child: Child,
    identity: ProcessIdentity,
    tree: CapturedProcessTree,
    failure: Option<ExplicitBootstrapExecFailureProof>,
    aliases: Option<PtyMasterAliasesClosed>,
}

impl Fixture {
    fn new() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let pair = native_pty_system().openpty(pty_size()).unwrap();
        let mut proof = PtySessionProof::capture_bootstrap_bound(
            pair.master.as_ref(),
            TEST_CAPABILITY.to_owned(),
        )
        .unwrap();
        let failure = proof
            .take_bootstrap_exec_authority()
            .unwrap()
            .confirm_explicit_failure()
            .unwrap();
        proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
        let release = proof.bind_master_alias_release(identity).unwrap();
        let tree = CapturedProcessTree::capture_pty(identity, proof).unwrap();
        Self {
            child,
            identity,
            tree,
            failure: Some(failure),
            aliases: Some(release.prove_closed_for_test()),
        }
    }

    fn record_failure(&mut self) {
        self.tree
            .record_explicit_bootstrap_exec_failure(self.failure.take().unwrap())
            .unwrap();
    }

    fn mark_root_exited_unreaped(&mut self) {
        let proof = self.tree.root_exit_proof_for_test(self.identity);
        self.tree.mark_root_exited_unreaped(proof).unwrap();
    }

    fn take_aliases(&mut self) -> PtyMasterAliasesClosed {
        self.aliases.take().unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}

#[test]
fn recorded_failure_requires_an_exact_unreaped_root_exit() {
    let mut fixture = Fixture::new();
    fixture.record_failure();
    let aliases = fixture.take_aliases();

    assert!(
        fixture
            .tree
            .prepare_explicit_bootstrap_failure_retirement(aliases)
            .is_err()
    );
    assert!(fixture.tree.explicit_bootstrap_exec_failure_is_pending());
    assert!(!fixture.tree.session_is_retired());
}

#[test]
fn foreign_master_closure_proof_cannot_prepare_retirement() {
    let mut fixture = Fixture::new();
    let mut foreign = Fixture::new();
    fixture.record_failure();
    fixture.mark_root_exited_unreaped();
    let aliases = foreign.take_aliases();

    assert!(
        fixture
            .tree
            .prepare_explicit_bootstrap_failure_retirement(aliases)
            .is_err()
    );
    assert!(fixture.tree.explicit_bootstrap_exec_failure_is_pending());
    assert!(!fixture.tree.session_is_retired());
}

#[test]
fn unrecorded_failure_cannot_prepare_retirement() {
    let mut fixture = Fixture::new();
    fixture.mark_root_exited_unreaped();
    let aliases = fixture.take_aliases();

    assert!(
        fixture
            .tree
            .prepare_explicit_bootstrap_failure_retirement(aliases)
            .is_err()
    );
    assert!(!fixture.tree.session_is_retired());
}

#[test]
fn recorded_failure_cannot_be_replaced_or_replayed() {
    let mut fixture = Fixture::new();
    let mut foreign = Fixture::new();
    fixture.record_failure();

    assert!(
        fixture
            .tree
            .record_explicit_bootstrap_exec_failure(foreign.failure.take().unwrap())
            .is_err()
    );
    assert!(fixture.tree.explicit_bootstrap_exec_failure_is_pending());
}

#[test]
fn typed_ready_commits_explicit_failure_retirement() {
    let mut fixture = Fixture::new();
    fixture.record_failure();
    fixture.mark_root_exited_unreaped();
    let aliases = fixture.take_aliases();

    fixture
        .tree
        .prepare_explicit_bootstrap_failure_retirement(aliases)
        .unwrap()
        .commit();

    assert!(fixture.tree.session_is_retired());
    assert!(!fixture.tree.explicit_bootstrap_exec_failure_is_pending());
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
