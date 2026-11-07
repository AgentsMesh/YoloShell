use super::*;
use anyhow::{Context, Result, ensure};
use std::os::fd::AsRawFd;

#[cfg(test)]
pub(crate) use crate::local_unix::macos::received_descriptors;

pub(crate) fn receive_once(socket: libc::c_int) -> Result<MacosProcessCapabilityChildWriter> {
    let descriptor = crate::local_unix::macos::receive_exactly_one_right(
        socket,
        crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    )
    .context(DESCRIPTOR_FRAME_DETAIL)?;
    ensure!(
        descriptor.as_raw_fd() > libc::STDERR_FILENO,
        DESCRIPTOR_FRAME_DETAIL
    );
    MacosProcessCapabilityChildWriter::from_transferred(descriptor)
}

#[cfg(test)]
pub(crate) mod receive_test_hook {
    pub(crate) use crate::local_unix::test_support::install;
}
