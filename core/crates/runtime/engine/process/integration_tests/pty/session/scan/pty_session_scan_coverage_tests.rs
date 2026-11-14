use super::*;

#[test]
fn writer_settlement_does_not_erase_completed_scan_coverage() {
    let mut result = CapabilityScanResult {
        candidates: CapabilityCandidates::default(),
        advance: CapabilityScanAdvance::reset(),
        scheduling_status: CapabilityScanStatus::Complete,
        coverage_status: CapabilityScanStatus::Complete,
    };

    require_writer_settlement(&mut result, true);

    assert_eq!(result.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(result.scheduling_status, CapabilityScanStatus::Complete);
    assert_eq!(result.coverage_status, CapabilityScanStatus::Complete);
}
