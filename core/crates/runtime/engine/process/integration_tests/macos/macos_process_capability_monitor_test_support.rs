use super::*;
use std::time::{Duration, Instant};

pub(super) const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(10);
const OBSERVATION_POLL_INTERVAL: Duration = Duration::from_millis(1);

pub(super) fn wait_for(
    reader: &MacosProcessCapabilityManagerReader,
    expected: MacosProcessCapabilityObservation,
) {
    let deadline = Instant::now() + OBSERVATION_TIMEOUT;
    while Instant::now() < deadline {
        if reader.liveness_observation() == Some(expected) {
            return;
        }
        std::thread::sleep(OBSERVATION_POLL_INTERVAL);
    }
    assert_eq!(reader.liveness_observation(), Some(expected));
}
