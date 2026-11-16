use anyhow::{Context, Result, ensure};
use std::time::Duration;

#[cfg(test)]
const DEADLINE_TEST_INTERVAL: Duration = Duration::from_secs(1);

pub fn monotonic_time_ns() -> Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) };
    ensure!(status == 0, std::io::Error::last_os_error());
    let seconds = u64::try_from(value.tv_sec).context("monotonic seconds are negative")?;
    let nanos = u64::try_from(value.tv_nsec).context("monotonic nanoseconds are negative")?;
    seconds
        .checked_mul(crate::runtime_constants::NANOSECONDS_PER_SECOND)
        .and_then(|base| base.checked_add(nanos))
        .context("monotonic time overflow")
}

pub fn monotonic_deadline_ns(timeout: Duration) -> Result<u64> {
    let timeout = u64::try_from(timeout.as_nanos()).context("monotonic timeout overflow")?;
    monotonic_time_ns()?
        .checked_add(timeout)
        .context("monotonic deadline overflow")
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/monotonic_inline_tests.rs"]
mod tests;
