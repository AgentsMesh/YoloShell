pub(crate) const DIAGNOSTIC: &[u8] =
    b"Yolo local Unix ancillary data was truncated; terminating for descriptor safety.\n";

pub(super) fn terminate_if_truncated(flags: libc::c_int) {
    if flags & libc::MSG_CTRUNC == libc::c_int::default() {
        return;
    }
    unsafe {
        let original = libc::fcntl(libc::STDERR_FILENO, libc::F_GETFL);
        if original >= libc::c_int::default() {
            libc::fcntl(
                libc::STDERR_FILENO,
                libc::F_SETFL,
                original | libc::O_NONBLOCK,
            );
        }
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
        libc::write(
            libc::STDERR_FILENO,
            DIAGNOSTIC.as_ptr().cast(),
            DIAGNOSTIC.len(),
        );
        libc::_exit(libc::EXIT_FAILURE);
    }
}
