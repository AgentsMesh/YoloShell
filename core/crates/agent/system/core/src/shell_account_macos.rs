use super::super::constants::{
    ACCOUNT_LOOKUP_MAX_INTERRUPT_RETRIES, ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES,
    ACCOUNT_RECORD_BUFFER_GROWTH_FACTOR, ACCOUNT_RECORD_BUFFER_MAX_BYTES,
    ACCOUNT_RECORD_BUFFER_MIN_BYTES,
};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::ptr;

const ACCOUNT_LOOKUP_SUCCESS: libc::c_int = 0;
const RETRY_COUNT_STEP: usize = 1;

pub(super) fn current_user_login_shell() -> Option<OsString> {
    let user_id = unsafe { libc::getuid() };
    login_shell_with(
        user_id,
        initial_buffer_bytes(),
        |user_id, record, buffer, result| unsafe {
            libc::getpwuid_r(user_id, record, buffer.as_mut_ptr(), buffer.len(), result)
        },
    )
}

fn initial_buffer_bytes() -> usize {
    let suggested = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    normalize_initial_buffer_bytes(suggested)
}

fn normalize_initial_buffer_bytes(suggested: libc::c_long) -> usize {
    usize::try_from(suggested)
        .ok()
        .filter(|size| *size >= ACCOUNT_RECORD_BUFFER_MIN_BYTES)
        .unwrap_or(ACCOUNT_RECORD_BUFFER_DEFAULT_BYTES)
        .min(ACCOUNT_RECORD_BUFFER_MAX_BYTES)
}

fn login_shell_with<F>(user_id: libc::uid_t, buffer_bytes: usize, mut lookup: F) -> Option<OsString>
where
    F: FnMut(
        libc::uid_t,
        &mut libc::passwd,
        &mut [libc::c_char],
        &mut *mut libc::passwd,
    ) -> libc::c_int,
{
    let size = buffer_bytes.clamp(
        ACCOUNT_RECORD_BUFFER_MIN_BYTES,
        ACCOUNT_RECORD_BUFFER_MAX_BYTES,
    );
    let mut buffer = vec![libc::c_char::default(); size];
    let mut interrupted = usize::default();
    loop {
        let mut record = blank_record();
        let mut result = ptr::null_mut();
        let status = lookup(user_id, &mut record, &mut buffer, &mut result);
        if status == ACCOUNT_LOOKUP_SUCCESS {
            return copy_shell(&record, result, &buffer);
        }
        if status == libc::EINTR && interrupted < ACCOUNT_LOOKUP_MAX_INTERRUPT_RETRIES {
            interrupted = interrupted.saturating_add(RETRY_COUNT_STEP);
            continue;
        }
        if status != libc::ERANGE || !grow_buffer(&mut buffer) {
            return None;
        }
    }
}

fn blank_record() -> libc::passwd {
    unsafe { std::mem::zeroed() }
}

fn grow_buffer(buffer: &mut Vec<libc::c_char>) -> bool {
    let next = buffer
        .len()
        .saturating_mul(ACCOUNT_RECORD_BUFFER_GROWTH_FACTOR)
        .min(ACCOUNT_RECORD_BUFFER_MAX_BYTES);
    if next == buffer.len() {
        return false;
    }
    buffer.resize(next, libc::c_char::default());
    true
}

fn copy_shell(
    record: &libc::passwd,
    result: *mut libc::passwd,
    buffer: &[libc::c_char],
) -> Option<OsString> {
    if result != ptr::from_ref(record).cast_mut() || record.pw_shell.is_null() {
        return None;
    }
    let offset = (record.pw_shell as usize).checked_sub(buffer.as_ptr() as usize)?;
    let shell = buffer.get(offset..)?;
    let end = shell
        .iter()
        .position(|byte| *byte == libc::c_char::default())?;
    Some(OsString::from_vec(
        shell[..end].iter().map(|byte| *byte as u8).collect(),
    ))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/shell_account_macos_tests.rs"]
mod tests;
