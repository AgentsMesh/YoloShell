use super::constants::{EXTRACTION_TIMEOUT, EXTRACTION_TIMEOUT_MESSAGE, PROCESS_POLL_INTERVAL};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Instant,
};
use yolo_release_contracts::{DITTO_ARGUMENTS, DITTO_EXECUTABLE};

pub fn extract(archive: &Path, destination: &Path) {
    let deadline = Instant::now() + EXTRACTION_TIMEOUT;
    let child = Command::new(DITTO_EXECUTABLE)
        .args(DITTO_ARGUMENTS)
        .arg(archive)
        .arg(destination)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut child = ScopedChild(child);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "ditto extraction failed: {status}");
            return;
        }
        assert!(Instant::now() < deadline, "{EXTRACTION_TIMEOUT_MESSAGE}");
        std::thread::sleep(
            PROCESS_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

struct ScopedChild(Child);

impl Drop for ScopedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
