use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn deadline_is_strictly_after_its_anchor() {
    let before = monotonic_time_ns().unwrap();
    let deadline = monotonic_deadline_ns(DEADLINE_TEST_INTERVAL).unwrap();
    assert!(deadline > before);
}
