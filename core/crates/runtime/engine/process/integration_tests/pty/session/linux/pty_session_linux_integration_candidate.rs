use super::super::LiveProcessIdentity;
use super::super::pty_session_proof::{
    LinuxPtyCustodyScanner, LinuxPtyCustodyState, LinuxPtyHolderState,
};
use super::support::{CandidateFixture, TestPty, candidate_matches};

#[test]
fn exact_capability_without_the_slave_device_is_excluded() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    pty.close_slave();
    let candidate = CandidateFixture::spawn(Some(&capability));

    assert!(!candidate_matches(&pty.proof, candidate.identity));
}

#[test]
fn exact_slave_device_without_the_capability_is_excluded() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    let candidate = CandidateFixture::spawn(None);
    pty.close_slave();

    assert!(!candidate_matches(&pty.proof, candidate.identity));
}

#[test]
fn capability_only_member_is_not_a_pty_holder() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    pty.close_slave();
    let candidate = CandidateFixture::spawn(Some(&capability));
    let live = LiveProcessIdentity::capture(candidate.identity)
        .unwrap()
        .unwrap();

    assert_eq!(
        pty.proof.linux_holder_state(&live),
        LinuxPtyHolderState::Excluded
    );
    assert_eq!(
        LinuxPtyCustodyScanner::new(&pty.proof).classify(&live),
        LinuxPtyCustodyState::Survivor
    );
}

#[test]
fn exact_pty_holder_without_capability_is_detected() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    let candidate = CandidateFixture::spawn(None);
    pty.close_slave();
    let live = LiveProcessIdentity::capture(candidate.identity)
        .unwrap()
        .unwrap();

    assert_eq!(
        pty.proof.linux_holder_state(&live),
        LinuxPtyHolderState::Holds
    );
    assert_eq!(
        LinuxPtyCustodyScanner::new(&pty.proof).classify(&live),
        LinuxPtyCustodyState::UntrustedHolder
    );
}

#[test]
fn reused_process_identity_is_indeterminate() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    let candidate = CandidateFixture::spawn(None);
    pty.close_slave();
    let live = LiveProcessIdentity::capture(candidate.identity)
        .unwrap()
        .unwrap()
        .with_generation(candidate.identity.start_time.wrapping_add(1));

    assert_eq!(
        pty.proof.linux_holder_state(&live),
        LinuxPtyHolderState::Indeterminate
    );
    assert_eq!(
        LinuxPtyCustodyScanner::new(&pty.proof).classify(&live),
        LinuxPtyCustodyState::Indeterminate
    );
}

#[test]
fn exact_pty_holder_with_capability_is_a_custody_target() {
    let directory = tempfile::tempdir().unwrap();
    let capability = directory.path().display().to_string();
    let mut pty = TestPty::open(&capability);
    let candidate = CandidateFixture::spawn(Some(&capability));
    pty.close_slave();
    let live = LiveProcessIdentity::capture(candidate.identity)
        .unwrap()
        .unwrap();

    assert_eq!(
        LinuxPtyCustodyScanner::new(&pty.proof).classify(&live),
        LinuxPtyCustodyState::Target
    );
}
