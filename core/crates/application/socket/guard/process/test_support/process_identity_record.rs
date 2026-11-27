use super::constants::IDENTITY_FIELD_SEPARATOR;
use std::path::Path;
use yolo_process_test_support::ProcessIdentity;

pub(super) fn write(path: &Path) {
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = format!(
        "{}{}{}",
        identity.pid, IDENTITY_FIELD_SEPARATOR, identity.start_time
    );
    std::fs::write(path, record).unwrap();
}

pub(super) fn read(path: &Path) -> Option<ProcessIdentity> {
    let record = std::fs::read_to_string(path).ok()?;
    let (pid, start_time) = record.split_once(IDENTITY_FIELD_SEPARATOR)?;
    Some(ProcessIdentity {
        pid: pid.parse().ok()?,
        start_time: start_time.parse().ok()?,
    })
}
