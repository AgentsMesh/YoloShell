use crate::constants::{UNKNOWN_MARKER_INCREMENT, UNKNOWN_MARKER_INITIAL};
use std::sync::atomic::{AtomicU64, Ordering};

// An intentionally nonstandard section: any neighbor-name allowlist is unsafe.
#[used]
#[unsafe(no_mangle)]
#[unsafe(link_section = "__DATA,__yolo_unknown")]
pub static YOLO_COVERAGE_UNKNOWN_DATA: AtomicU64 = AtomicU64::new(UNKNOWN_MARKER_INITIAL);

pub fn verify_and_advance() {
    let marker = std::hint::black_box(&YOLO_COVERAGE_UNKNOWN_DATA);
    assert_eq!(marker.load(Ordering::SeqCst), UNKNOWN_MARKER_INITIAL);
    assert_eq!(
        marker.fetch_add(UNKNOWN_MARKER_INCREMENT, Ordering::SeqCst),
        UNKNOWN_MARKER_INITIAL
    );
    assert_eq!(
        marker.load(Ordering::SeqCst),
        UNKNOWN_MARKER_INITIAL + UNKNOWN_MARKER_INCREMENT
    );
}
