use crate::fake_agent_constants::NATIVE_CONTROL_SIGNAL_MARKER_ENV;
use std::{
    fs::{File, OpenOptions},
    io,
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::PathBuf,
    sync::atomic::{AtomicI32, Ordering},
};

const NO_DESCRIPTOR: i32 = -1;
const PRIVATE_MODE: u32 = 0o600;
const SIGNAL_MARKER: &[u8] = b"I";
const INVALID_MARKER: &str = "native control fixture marker must be absolute";
static MARKER_DESCRIPTOR: AtomicI32 = AtomicI32::new(NO_DESCRIPTOR);

/// Enabled only for the test agent invocation. One append is signal-safe and
/// leaves provider hooks untouched, so E2E can distinguish delivery from state.
pub(crate) fn install_if_configured() -> io::Result<Option<File>> {
    let Some(path) = std::env::var_os(NATIVE_CONTROL_SIGNAL_MARKER_ENV) else {
        return Ok(None);
    };
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, INVALID_MARKER));
    }
    let file = OpenOptions::new()
        .create_new(true)
        .append(true)
        .mode(PRIVATE_MODE)
        .open(path)?;
    MARKER_DESCRIPTOR.store(file.as_raw_fd(), Ordering::Release);
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = observe_interrupt as *const () as usize;
    action.sa_flags = libc::SA_RESTART;
    if unsafe { libc::sigemptyset(&mut action.sa_mask) } != i32::default()
        || unsafe { libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()) } != i32::default()
    {
        MARKER_DESCRIPTOR.store(NO_DESCRIPTOR, Ordering::Release);
        return Err(io::Error::last_os_error());
    }
    Ok(Some(file))
}

extern "C" fn observe_interrupt(signal: libc::c_int) {
    let descriptor = MARKER_DESCRIPTOR.load(Ordering::Acquire);
    if signal == libc::SIGINT && descriptor != NO_DESCRIPTOR {
        // write(2) is async-signal-safe; the marker is one O_APPEND byte.
        let _ = unsafe {
            libc::write(
                descriptor,
                SIGNAL_MARKER.as_ptr().cast(),
                SIGNAL_MARKER.len(),
            )
        };
    }
}
