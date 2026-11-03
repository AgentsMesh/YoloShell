use anyhow::Result;

pub(crate) fn send_writer_once(socket: libc::c_int, descriptor: libc::c_int) -> Result<()> {
    crate::local_unix::test_support::send_right(
        socket,
        descriptor,
        crate::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    );
    Ok(())
}

pub(crate) fn receive_writer_once(
    socket: libc::c_int,
) -> Result<crate::MacosProcessCapabilityChildWriter> {
    let descriptor = crate::local_unix::macos::receive_exactly_one_right(
        socket,
        crate::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    )?;
    crate::MacosProcessCapabilityChildWriter::from_transferred(descriptor)
}
