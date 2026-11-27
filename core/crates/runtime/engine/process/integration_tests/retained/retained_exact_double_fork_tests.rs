use super::retained_exact_double_fork_support::DoubleForkFixture;
use super::{PtySessionProof, RetainedExactProcess, RetainedProcessObservation};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const FORBIDDEN_REFRESH_DETAIL: &str = "PTY exact observer performed a global refresh";
const FORBIDDEN_SIGNAL_DETAIL: &str = "PTY exact observer attempted process signalling";
const SIGNAL_INCREMENT: usize = 1;

#[test]
fn pty_proof_without_object_lease_never_signals() {
    let mut fixture = DoubleForkFixture::spawn(false);
    let mut retained =
        RetainedExactProcess::capture_pty(fixture.root_identity, fixture.proof.clone())
            .unwrap()
            .unwrap();
    assert!(retained.is_exact_observer_for_test());
    let escaped = fixture.release_leaf();
    retained.fail_next_refresh_for_test(FORBIDDEN_REFRESH_DETAIL);

    assert_eq!(
        retained.observe().unwrap(),
        RetainedProcessObservation::Running
    );
    assert_no_signal(&mut retained, |target| target.begin_containment());
    assert_no_signal(&mut retained, |target| target.enforce_containment());
    assert!(fixture.root_identity.matches());
    assert!(escaped.matches());
}

#[test]
fn pty_exact_observer_ignores_injected_refresh_failure() {
    let fixture = DoubleForkFixture::spawn(false);
    let mut retained =
        RetainedExactProcess::capture_pty(fixture.root_identity, fixture.proof.clone())
            .unwrap()
            .unwrap();
    retained.fail_next_refresh_for_test(FORBIDDEN_REFRESH_DETAIL);

    assert_eq!(
        retained.observe().unwrap(),
        RetainedProcessObservation::Running
    );
    assert!(fixture.root_identity.matches());
}

#[test]
fn root_exit_with_a_late_escaped_descendant_is_indeterminate() {
    let mut fixture = DoubleForkFixture::spawn(true);
    let mut retained =
        RetainedExactProcess::capture_pty(fixture.root_identity, fixture.proof.clone())
            .unwrap()
            .unwrap();
    let escaped = fixture.release_leaf();
    fixture.terminate_root();

    assert_eq!(
        retained.observe().unwrap(),
        RetainedProcessObservation::Indeterminate
    );
    assert!(escaped.matches());
}

#[test]
fn discovery_and_no_proof_custody_keep_refresh_semantics() {
    let fixture = DoubleForkFixture::spawn(false);
    let mut exact = RetainedExactProcess::capture(fixture.root_identity)
        .unwrap()
        .unwrap();
    assert!(!exact.is_exact_observer_for_test());
    assert_refresh_failure(&mut exact);

    let discovery = PtySessionProof::discovery(uuid::Uuid::new_v4().to_string()).unwrap();
    let mut discovered = RetainedExactProcess::capture_pty(fixture.root_identity, discovery)
        .unwrap()
        .unwrap();
    assert!(!discovered.is_exact_observer_for_test());
    assert_refresh_failure(&mut discovered);
}

fn assert_no_signal(
    retained: &mut RetainedExactProcess,
    request: impl FnOnce(&mut RetainedExactProcess) -> anyhow::Result<bool>,
) {
    let signals = Arc::new(AtomicUsize::new(usize::default()));
    let observed = Arc::clone(&signals);
    let _guard = super::install_test_live_signal_failure(move || {
        observed.fetch_add(SIGNAL_INCREMENT, Ordering::AcqRel);
        Some(FORBIDDEN_SIGNAL_DETAIL.to_owned())
    });
    assert!(!request(retained).unwrap());
    assert_eq!(signals.load(Ordering::Acquire), usize::default());
}

fn assert_refresh_failure(retained: &mut RetainedExactProcess) {
    retained.fail_next_refresh_for_test(FORBIDDEN_REFRESH_DETAIL);
    let error = retained.observe().unwrap_err();
    assert!(format!("{error:#}").contains(FORBIDDEN_REFRESH_DETAIL));
}
