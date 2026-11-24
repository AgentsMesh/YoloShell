/// A local Unix endpoint necessarily names a Hub on this machine, so its host
/// name is a truthful display label. Remote endpoints use the peer-configuration
/// label when the entry Hub has one, otherwise the compact Hub identity.
pub fn local_hostname() -> Option<String> {
    let mut buffer = [0_u8; HOST_NAME_CAPACITY];
    // SAFETY: `buffer` is writable for `buffer.len()` bytes for the duration of
    // the call, which is the contract required by `gethostname`.
    if unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) } != 0 {
        return None;
    }
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    let name = std::str::from_utf8(&buffer[..end]).ok()?.trim();
    (!name.is_empty() && !name.chars().any(char::is_control)).then(|| name.to_owned())
}

const HOST_NAME_CAPACITY: usize = 256;
