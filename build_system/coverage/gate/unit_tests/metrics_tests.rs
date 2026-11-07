use crate::constants::{BRANCHES_LABEL, FUNCTIONS_LABEL, LINES_LABEL, REGIONS_LABEL};
use crate::metrics::{Metric, Totals};

#[test]
fn threshold_is_strictly_greater_than_ninety_five_percent() {
    assert!(!Metric::new(100, 95, LINES_LABEL).unwrap().passes());
    assert!(Metric::new(100, 96, LINES_LABEL).unwrap().passes());
}

#[test]
fn ratio_comparison_does_not_overflow() {
    let metric = Metric::new(u64::MAX, u64::MAX, LINES_LABEL).unwrap();
    assert!(metric.passes());
}

#[test]
fn invalid_and_overflowing_metrics_are_rejected() {
    assert!(Metric::new(1, 2, LINES_LABEL).is_err());
    let maximum = Metric::new(u64::MAX, u64::MAX, LINES_LABEL).unwrap();
    let one = Metric::new(1, 1, LINES_LABEL).unwrap();
    assert!(maximum.checked_add(one, LINES_LABEL).is_err());
}

#[test]
fn totals_require_all_denominators_and_thresholds() {
    let covered = Metric::new(100, 96, LINES_LABEL).unwrap();
    let threshold = Metric::new(100, 95, FUNCTIONS_LABEL).unwrap();
    assert!(Totals::new(Metric::default(), covered, covered, covered).is_err());
    assert!(Totals::new(covered, Metric::default(), covered, covered).is_err());
    assert!(Totals::new(covered, covered, Metric::default(), covered).is_err());
    assert!(Totals::new(covered, covered, covered, Metric::default()).is_err());
    assert!(
        !Totals::new(covered, threshold, covered, covered)
            .unwrap()
            .passes()
    );
    assert!(!Metric::new(100, 95, REGIONS_LABEL).unwrap().passes());
    assert!(!Metric::new(100, 95, BRANCHES_LABEL).unwrap().passes());
}
