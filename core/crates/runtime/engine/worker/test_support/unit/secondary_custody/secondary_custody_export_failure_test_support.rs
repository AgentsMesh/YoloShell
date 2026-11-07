#[cfg(target_os = "macos")]
const EXPORT_SOCKET_FILL_CHUNK_BYTES: usize = 16 * 1_024;
#[cfg(target_os = "macos")]
const EXPORT_SOCKET_SYSTEM_FAILURE: libc::ssize_t = -1;

#[cfg(target_os = "macos")]
fn install_successor_export_source(
    fixture: &Fixture,
) -> (
    Arc<WorkerSecondaryCustodySource>,
    portable_pty::PtyPair,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    let record = lock_recover(&fixture.state.record).clone();
    let (source, pair, writer) = custody_source_for_test(&record);
    source.complete_startup_transfer();
    let source = Arc::new(source);
    let previous = lock_recover(&fixture.state.secondary_custody).replace(Arc::clone(&source));
    assert!(previous.is_none());
    (source, pair, writer)
}

#[cfg(target_os = "macos")]
fn saturate_successor_export_socket(stream: &crate::local_unix::LocalUnixStream) {
    use std::os::fd::AsRawFd;

    let raw = stream.as_raw_fd();
    let chunk = [u8::default(); EXPORT_SOCKET_FILL_CHUNK_BYTES];
    loop {
        let sent = unsafe {
            libc::send(
                raw,
                chunk.as_ptr().cast(),
                chunk.len(),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if sent == EXPORT_SOCKET_SYSTEM_FAILURE {
            let error = std::io::Error::last_os_error();
            assert!(
                error.raw_os_error() == Some(libc::EAGAIN)
                    || error.raw_os_error() == Some(libc::EWOULDBLOCK)
            );
            return;
        }
        assert!(sent > libc::ssize_t::default());
    }
}

#[cfg(target_os = "macos")]
fn shutdown_successor_export_writes(stream: &crate::local_unix::LocalUnixStream) {
    use std::os::fd::AsRawFd;

    assert_eq!(
        unsafe { libc::shutdown(stream.as_raw_fd(), libc::SHUT_WR) },
        libc::c_int::default()
    );
}

#[cfg(target_os = "macos")]
async fn assert_failed_export_settled(
    fixture: &Fixture,
    source: &WorkerSecondaryCustodySource,
    holder: crate::pty_custody::CustodyHolderIdentity,
) {
    assert!(holder.matches());
    assert_eq!(
        source.successor_export_counts_for_test(),
        NO_SUCCESSOR_EXPORTS
    );
    assert_eq!(
        poll_export_fenced_alias_closure(fixture).await,
        pty_runtime_alias_custody::PtyRuntimeAliasClosePoll::Closed
    );
    assert!(holder.matches());
}
