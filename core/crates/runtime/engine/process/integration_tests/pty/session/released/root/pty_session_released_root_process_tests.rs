use super::*;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use std::time::Instant;

#[path = "pty_session_released_root_process_fixture.rs"]
mod fixture;
use fixture::{RootPtyBinding, RootScanFixture};

const ROOT_AUTHORITY_DETAIL: &str = "released PTY root exact authority was not retained";
const ROOT_PUBLISHED_DETAIL: &str = "released PTY root leaked into descendant evidence";
const AMBIENT_PROGRESS_DETAIL: &str = "ambient sweep completed before exact root authority";
const FOREIGN_ROOT_DETAIL: &str = "foreign PTY root obtained exact authority";
const FOREIGN_COVERAGE_DETAIL: &str = "foreign PTY scan did not reach conclusive coverage";

#[test]
fn released_pty_root_bypasses_the_ambient_descriptor_sweep() {
    let fixture = RootScanFixture::spawn(RootPtyBinding::Exact);
    let mut state = ReleasedScanState::default();
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let scan = fixture.scan(&state);
        assert_eq!(scan.coverage_status, CapabilityScanStatus::Pending);
        assert_not_published(&scan.candidates, ROOT_PUBLISHED_DETAIL);
        let next = scan.advance.next.released;
        if next.covers_exact_closure(&[fixture.root_live()]) {
            assert_eq!(
                next.pending_lineages_for_test(),
                [fixture.ambient_lineage()],
                "{AMBIENT_PROGRESS_DETAIL}"
            );
            return;
        }
        state = next;
        assert!(Instant::now() < deadline, "{ROOT_AUTHORITY_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn released_root_with_a_foreign_pty_remains_unauthorized() {
    let fixture = RootScanFixture::spawn(RootPtyBinding::Foreign);
    let mut state = ReleasedScanState::default();
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let scan = fixture.scan(&state);
        assert_not_published(&scan.candidates, FOREIGN_ROOT_DETAIL);
        let coverage_complete = scan.coverage_status == CapabilityScanStatus::Complete;
        state = scan.advance.next.released;
        assert!(
            !state.covers_exact_closure(&[fixture.root_live()]),
            "{FOREIGN_ROOT_DETAIL}"
        );
        if coverage_complete {
            return;
        }
        assert!(Instant::now() < deadline, "{FOREIGN_COVERAGE_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn assert_not_published(candidates: &CapabilityCandidates, detail: &str) {
    assert!(candidates.observed.is_empty(), "{detail}");
    assert!(candidates.stable.is_empty(), "{detail}");
    assert!(candidates.identities.is_empty(), "{detail}");
}
