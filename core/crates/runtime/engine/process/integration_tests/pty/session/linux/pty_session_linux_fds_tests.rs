use super::super::linux_contract::BudgetKind;
use super::*;
use std::os::unix::fs::MetadataExt;

const LARGE_WINDOW: usize = 4_096;

#[test]
fn descriptor_scan_matches_the_underlying_character_device() {
    let device = std::fs::File::open("/dev/null").unwrap();
    let expected = device.metadata().unwrap().rdev();
    let query = holds_device(
        std::process::id(),
        expected,
        WorkWindow {
            limit: LARGE_WINDOW,
            exhausted: BudgetKind::CandidateDescriptors,
        },
    );

    assert!(matches!(query.value, DescriptorValue::Holds));
}

#[test]
fn descriptor_scan_excludes_a_device_that_is_not_held() {
    let query = holds_device(
        std::process::id(),
        u64::MAX,
        WorkWindow {
            limit: LARGE_WINDOW,
            exhausted: BudgetKind::CandidateDescriptors,
        },
    );

    assert!(matches!(query.value, DescriptorValue::Excluded));
}

#[test]
fn zero_descriptor_budget_is_pending() {
    let query = holds_device(
        std::process::id(),
        u64::default(),
        WorkWindow {
            limit: usize::default(),
            exhausted: BudgetKind::BatchDescriptors,
        },
    );

    assert!(matches!(
        query.value,
        DescriptorValue::Pending(CandidatePending::Budget(BudgetKind::BatchDescriptors))
    ));
}
