use super::*;

pub(crate) const MASTER_DESCRIPTOR_CONTEXT: &str = "validate Linux PTY master output descriptor";

pub(crate) fn output_queue_state(file: &File) -> Result<OutputQueueState> {
    let mut number = libc::c_uint::default();
    // SAFETY: TIOCGPTN reads the retained Unix98 master identity into a writable word.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            libc::TIOCGPTN,
            std::ptr::addr_of_mut!(number),
        )
    };
    if result == SYSCALL_FAILURE {
        return Err(io::Error::last_os_error()).context(MASTER_DESCRIPTOR_CONTEXT);
    }
    // N_TTY poll flushes pending flip-buffer work before reporting an empty queue.
    // The final-reader probe remains required after this observation.
    poll_events(file).context(MASTER_QUEUE_CONTEXT)?;
    let queued = readable_bytes(file).context(MASTER_QUEUE_CONTEXT)?;
    Ok(if queued == libc::c_int::default() {
        OutputQueueState::Empty
    } else {
        OutputQueueState::Pending
    })
}
