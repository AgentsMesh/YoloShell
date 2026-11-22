pub(super) fn preserve(error: &anyhow::Error) -> bool {
    let mut saw_io = false;
    for cause in error.chain() {
        let Some(io) = cause.downcast_ref::<std::io::Error>() else {
            continue;
        };
        saw_io = true;
        if changed_or_unsafe(io) {
            return true;
        }
    }
    !saw_io
}

#[cfg(test)]
fn classify_lock_error(error: std::io::Error) -> anyhow::Result<bool> {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        Ok(true)
    } else {
        Err(error.into())
    }
}

fn changed_or_unsafe(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::IsADirectory
            | std::io::ErrorKind::NotADirectory
            | std::io::ErrorKind::InvalidInput
            | std::io::ErrorKind::InvalidData
    ) || matches!(
        error.raw_os_error(),
        Some(libc::ELOOP) | Some(libc::EISDIR) | Some(libc::ENOTDIR) | Some(libc::ENOTEMPTY)
    )
}

pub(super) fn preserve_io(error: &std::io::Error) -> bool {
    changed_or_unsafe(error)
}

pub(super) fn process_alive(pid: libc::pid_t) -> bool {
    if unsafe { libc::kill(pid, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(test)]
pub(super) fn preserve_errno(code: libc::c_int) -> bool {
    preserve(&std::io::Error::from_raw_os_error(code).into())
}

#[cfg(test)]
pub(super) fn lock_errno(code: libc::c_int) -> anyhow::Result<bool> {
    classify_lock_error(std::io::Error::from_raw_os_error(code))
}
