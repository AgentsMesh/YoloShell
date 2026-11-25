use super::{PtyProcessCustody, PtyProcessCustodyProgress};
use crate::process::CapabilityScanActivity;

#[test]
fn custody_phase_transition_reports_cleanup_progress() {
    let identity = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(identity).unwrap();
    tree.record_capability_scan_activity_for_test(CapabilityScanActivity::ContinueRound);
    let tracking = PtyProcessCustody::tracking(tree);
    let tracking_phase = tracking.phase();
    let tree = match tracking {
        PtyProcessCustody::Tracking(tree) => tree,
        _ => unreachable!(),
    };
    let mut freezing = PtyProcessCustody::Freezing(tree.begin_force());

    assert_eq!(
        freezing.take_progress_after(Some(tracking_phase)),
        PtyProcessCustodyProgress {
            phase_changed: true,
            scan_activity: CapabilityScanActivity::ContinueRound,
        }
    );
    let freezing_phase = freezing.phase();
    assert_eq!(
        freezing.take_progress_after(Some(freezing_phase)),
        PtyProcessCustodyProgress {
            phase_changed: false,
            scan_activity: CapabilityScanActivity::Idle,
        }
    );
}

#[test]
fn cleanup_handoff_discards_progress_from_before_its_ownership() {
    let identity = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(identity).unwrap();
    tree.record_capability_scan_activity_for_test(CapabilityScanActivity::Evidence);
    let mut custody = PtyProcessCustody::tracking(tree);

    custody.clear_stale_progress();

    let cleanup_phase = custody.phase();
    assert_eq!(
        custody.take_progress_after(Some(cleanup_phase)),
        PtyProcessCustodyProgress {
            phase_changed: false,
            scan_activity: CapabilityScanActivity::Idle,
        }
    );
}

#[test]
fn scan_continuation_remains_distinct_from_semantic_evidence() {
    let identity = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(identity).unwrap();
    tree.record_capability_scan_activity_for_test(CapabilityScanActivity::ContinueRound);
    let mut custody = PtyProcessCustody::tracking(tree);
    let phase = custody.phase();

    assert_eq!(
        custody.take_progress_after(Some(phase)),
        PtyProcessCustodyProgress {
            phase_changed: false,
            scan_activity: CapabilityScanActivity::ContinueRound,
        }
    );
}
