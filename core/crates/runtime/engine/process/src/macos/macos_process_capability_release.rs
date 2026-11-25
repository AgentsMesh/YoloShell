use super::{
    MacosProcessCapabilityManagerReader, MacosProcessCapabilityObservation,
    MacosProcessCapabilityRelease, RELEASE_DRAIN_BYTES, RELEASE_DRAIN_PASSES,
    RELEASE_PROTOCOL_DETAIL, RELEASE_READ_CONTEXT,
};
use anyhow::{Result, ensure};
use std::os::fd::AsRawFd;

pub(super) const PREMATURE_RELEASE_DETAIL: &str =
    "process capability writer released before the root exited";
pub(super) const INDETERMINATE_RELEASE_DETAIL: &str =
    "process capability release observation is indeterminate";

impl MacosProcessCapabilityManagerReader {
    pub(crate) fn poll_release(&self) -> Result<MacosProcessCapabilityRelease> {
        let monitor = self
            .monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(monitor) = monitor.as_ref() {
            return match monitor.observation() {
                MacosProcessCapabilityObservation::WriterPresent
                | MacosProcessCapabilityObservation::ReleasedPendingRootExit
                | MacosProcessCapabilityObservation::RootExited => {
                    Ok(MacosProcessCapabilityRelease::WriterPresent)
                }
                MacosProcessCapabilityObservation::ReleasedAfterRootExit => {
                    Ok(MacosProcessCapabilityRelease::Released)
                }
                MacosProcessCapabilityObservation::PrematurelyReleased => {
                    anyhow::bail!(PREMATURE_RELEASE_DETAIL)
                }
                MacosProcessCapabilityObservation::Indeterminate => {
                    anyhow::bail!(INDETERMINATE_RELEASE_DETAIL)
                }
            };
        }
        poll_descriptor_release(self.descriptor.as_raw_fd())
    }

    pub fn liveness_observation(&self) -> Option<MacosProcessCapabilityObservation> {
        self.monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(super::monitor::MacosProcessCapabilityMonitor::observation)
    }
}

pub(super) fn poll_descriptor_release(
    descriptor: libc::c_int,
) -> Result<MacosProcessCapabilityRelease> {
    let mut buffer = [u8::default(); RELEASE_DRAIN_BYTES];
    let mut unexpected_data = false;
    for _pass in 0..RELEASE_DRAIN_PASSES {
        let read = unsafe { libc::read(descriptor, buffer.as_mut_ptr().cast(), buffer.len()) };
        if read == libc::ssize_t::default() {
            ensure!(!unexpected_data, RELEASE_PROTOCOL_DETAIL);
            return Ok(MacosProcessCapabilityRelease::Released);
        }
        if read > libc::ssize_t::default() {
            unexpected_data = true;
            continue;
        }
        let error = std::io::Error::last_os_error();
        match error.kind() {
            std::io::ErrorKind::Interrupted => continue,
            std::io::ErrorKind::WouldBlock => {
                ensure!(!unexpected_data, RELEASE_PROTOCOL_DETAIL);
                return Ok(MacosProcessCapabilityRelease::WriterPresent);
            }
            _ => return Err(error).map_err(anyhow::Error::new),
        }
    }
    anyhow::bail!("{RELEASE_READ_CONTEXT}: {RELEASE_PROTOCOL_DETAIL}")
}
