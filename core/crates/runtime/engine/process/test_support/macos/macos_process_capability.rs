use super::*;
use std::os::fd::AsRawFd;

pub(in crate::process) fn transfer(
    issuer: MacosProcessCapabilityIssuer,
) -> Result<(
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
)> {
    issuer.into_staged()
}

pub(in crate::process) fn write_unexpected_byte(
    writer: &MacosProcessCapabilityChildWriter,
) -> Result<()> {
    const TEST_BYTE: u8 = b'!';
    const WRITTEN_BYTE_COUNT: libc::ssize_t = 1;
    let byte = TEST_BYTE;
    let written = unsafe {
        libc::write(
            writer.descriptor.as_raw_fd(),
            std::ptr::addr_of!(byte).cast(),
            std::mem::size_of_val(&byte),
        )
    };
    ensure!(written == WRITTEN_BYTE_COUNT, RELEASE_PROTOCOL_DETAIL);
    Ok(())
}
