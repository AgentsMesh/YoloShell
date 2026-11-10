mod constants;
mod profile_name;

use constants::*;
use profile_name::ProfileName;
use std::ffi::CStr;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;

pub fn discard_reaped_kill_profile(
    pid: u32,
    status: std::process::ExitStatus,
) -> io::Result<usize> {
    if status.signal() == Some(libc::SIGKILL) {
        discard_aborted_process_profile(pid)
    } else {
        Ok(usize::default())
    }
}

// The caller has already terminated and waited for this deliberately aborted fixture.
pub fn discard_aborted_process_profile(pid: u32) -> io::Result<usize> {
    if pid == u32::default() || pid == std::process::id() {
        return Err(profile_name::invalid());
    }
    let Some(pattern) = std::env::var_os(PROFILE_ENV) else {
        return Ok(usize::default());
    };
    discard_pattern(Path::new(&pattern), pid, &hostname()?)
}

fn discard_pattern(pattern: &Path, pid: u32, hostname: &str) -> io::Result<usize> {
    let parent = pattern.parent().ok_or_else(profile_name::invalid)?;
    let name = pattern
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(profile_name::invalid)?;
    let matcher = ProfileName::parse(name, pid, hostname)?;
    // Live-synced profiles remain evidence after a forced exit. The collector
    // must still reject any corruption, so never discard records in this mode.
    if matcher.continuous {
        return Ok(usize::default());
    }
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(usize::default()),
        Err(error) => return Err(error),
    };
    let mut removed = usize::default();
    for entry in entries {
        let entry = entry?;
        if !entry
            .file_name()
            .to_str()
            .is_some_and(|name| matcher.matches(name))
            || !entry.file_type()?.is_file()
        {
            continue;
        }
        match std::fs::remove_file(entry.path()) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(removed)
}

fn hostname() -> io::Result<String> {
    let mut buffer = [0_u8; HOSTNAME_BYTES];
    let status = unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    CStr::from_bytes_until_nul(&buffer)
        .map_err(|_| profile_name::invalid())?
        .to_str()
        .map(str::to_owned)
        .map_err(|_| profile_name::invalid())
}

#[cfg(test)]
#[path = "../unit_tests/profile_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../unit_tests/continuous_tests.rs"]
mod continuous_tests;
