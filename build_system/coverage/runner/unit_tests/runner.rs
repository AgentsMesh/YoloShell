#[path = "../test_support/constants.rs"]
mod constants;

use constants::*;

#[test]
fn collector_failure_probe() {
    assert!(
        yolo_coverage_protocol::production_key(file!())
            .unwrap()
            .is_none()
    );
    match std::env::var(FAILURE_PROBE_ENV).as_deref() {
        Ok(ENABLED) => {}
        Ok(PREEXISTING_MARKER_PROBE) => {
            let marker = std::env::var_os(PREMATURE_EXIT_ENV).expect(PREMATURE_EXIT_ENV);
            std::fs::write(marker, PREEXISTING_FAILURE).unwrap();
            return;
        }
        _ => return,
    }
    let directory = std::env::var_os(COVERAGE_DIRECTORY).expect(COVERAGE_DIRECTORY);
    std::fs::write(
        std::path::Path::new(&directory).join(CORRUPT_PROFILE),
        CORRUPT_PROFILE_CONTENT,
    )
    .unwrap();
}
