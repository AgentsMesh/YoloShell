use super::super::constants::{
    MACOS_PROC_ALL_PIDS, MAX_PTY_PROOF_CANDIDATES, PID_LIST_SENTINEL_COUNT,
};
use std::collections::HashSet;
use std::error::Error;
use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const PROC_LISTPIDSPATH_EXCLUDE_EVTONLY: c_uint = 2;
const PROC_ALL_PIDS_TYPEINFO: c_uint = 0;
const EMPTY_QUERY_RESULT: c_int = 0;
const QUERY_FAILURE_RESULT: c_int = -1;
const SUCCESS_ERRNO: c_int = 0;
const LIMIT_DETAIL: &str = "path-holder process inventory exceeds its budget";
const BUFFER_OVERFLOW_DETAIL: &str = "path-holder process buffer size overflows";
const BUFFER_RANGE_DETAIL: &str = "path-holder process buffer exceeds libproc range";
const ALLOCATION_DETAIL: &str = "path-holder process buffer allocation failed";
const SET_ALLOCATION_DETAIL: &str = "path-holder result allocation failed";
const PATH_DETAIL: &str = "path-holder query path must be nonempty and NUL-free";
const NEGATIVE_RESULT_DETAIL: &str = "proc_listpidspath returned an invalid negative result";
const MISSING_ERRNO_DETAIL: &str = "proc_listpidspath failed without setting errno";
const MISALIGNED_RESULT_DETAIL: &str = "proc_listpidspath returned a misaligned byte count";
const BUFFER_EXCEEDED_DETAIL: &str = "proc_listpidspath exceeded its supplied buffer";
const NONPOSITIVE_PID_DETAIL: &str = "proc_listpidspath returned a nonpositive process id";
const QUERY_ERROR_KIND: &str = "path-holder query failed";
const BUDGET_ERROR_KIND: &str = "path-holder budget exceeded";
const INVARIANT_ERROR_KIND: &str = "path-holder response invariant failed";

type RawPid = c_int;
pub(super) type PathHolderResult<T> = Result<T, PathHolderError>;

#[derive(Debug)]
pub(super) enum PathHolderError {
    Query(std::io::Error),
    BudgetExceeded(&'static str),
    Invariant(&'static str),
}

impl Display for PathHolderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Query(error) => write!(formatter, "{QUERY_ERROR_KIND}: {error}"),
            Self::BudgetExceeded(detail) => write!(formatter, "{BUDGET_ERROR_KIND}: {detail}"),
            Self::Invariant(detail) => write!(formatter, "{INVARIANT_ERROR_KIND}: {detail}"),
        }
    }
}

impl Error for PathHolderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Query(error) => Some(error),
            Self::BudgetExceeded(_) | Self::Invariant(_) => None,
        }
    }
}

impl PathHolderError {
    pub(super) fn is_missing_path(&self) -> bool {
        matches!(self, Self::Query(error) if error.raw_os_error() == Some(libc::ENOENT))
    }
}

pub(super) fn path_holders(path: &Path, limit: usize) -> PathHolderResult<HashSet<u32>> {
    path_holders_with(
        path,
        limit,
        |kind, selector, path, flags, buffer, bytes| unsafe {
            proc_listpidspath(kind, selector, path, flags, buffer, bytes)
        },
    )
}

fn path_holders_with<Q>(path: &Path, limit: usize, mut query: Q) -> PathHolderResult<HashSet<u32>>
where
    Q: FnMut(c_uint, c_uint, *const c_char, c_uint, *mut c_void, c_int) -> c_int,
{
    if limit > MAX_PTY_PROOF_CANDIDATES {
        return Err(PathHolderError::BudgetExceeded(LIMIT_DETAIL));
    }
    let path = checked_path(path)?;
    let (mut buffer, buffer_bytes) = pid_buffer(limit)?;
    reset_errno();
    let returned = query(
        MACOS_PROC_ALL_PIDS,
        PROC_ALL_PIDS_TYPEINFO,
        path.as_ptr(),
        PROC_LISTPIDSPATH_EXCLUDE_EVTONLY,
        buffer.as_mut_ptr().cast(),
        buffer_bytes,
    );
    parse_pids(&buffer, validate_query_result(returned)?, limit)
}

fn checked_path(path: &Path) -> PathHolderResult<CString> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.is_empty() {
        return Err(PathHolderError::Invariant(PATH_DETAIL));
    }
    CString::new(bytes).map_err(|_| PathHolderError::Invariant(PATH_DETAIL))
}

fn pid_buffer(limit: usize) -> PathHolderResult<(Vec<RawPid>, c_int)> {
    let capacity = limit
        .checked_add(PID_LIST_SENTINEL_COUNT)
        .ok_or(PathHolderError::BudgetExceeded(BUFFER_OVERFLOW_DETAIL))?;
    let bytes = capacity
        .checked_mul(size_of::<RawPid>())
        .ok_or(PathHolderError::BudgetExceeded(BUFFER_OVERFLOW_DETAIL))?;
    let bytes =
        c_int::try_from(bytes).map_err(|_| PathHolderError::BudgetExceeded(BUFFER_RANGE_DETAIL))?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(capacity)
        .map_err(|_| PathHolderError::BudgetExceeded(ALLOCATION_DETAIL))?;
    buffer.resize(capacity, RawPid::default());
    Ok((buffer, bytes))
}

fn validate_query_result(returned: c_int) -> PathHolderResult<usize> {
    if returned.is_positive() {
        return Ok(returned as usize);
    }
    if returned < QUERY_FAILURE_RESULT {
        return Err(PathHolderError::Invariant(NEGATIVE_RESULT_DETAIL));
    }
    if returned == EMPTY_QUERY_RESULT {
        return Ok(0);
    }
    let errno = current_errno();
    if errno == SUCCESS_ERRNO {
        return Err(PathHolderError::Invariant(MISSING_ERRNO_DETAIL));
    }
    Err(PathHolderError::Query(std::io::Error::from_raw_os_error(
        errno,
    )))
}

fn parse_pids(
    buffer: &[RawPid],
    returned_bytes: usize,
    limit: usize,
) -> PathHolderResult<HashSet<u32>> {
    if !returned_bytes.is_multiple_of(size_of::<RawPid>()) {
        return Err(PathHolderError::Invariant(MISALIGNED_RESULT_DETAIL));
    }
    let count = returned_bytes / size_of::<RawPid>();
    if count > buffer.len() {
        return Err(PathHolderError::Invariant(BUFFER_EXCEEDED_DETAIL));
    }
    if count > limit {
        return Err(PathHolderError::BudgetExceeded(LIMIT_DETAIL));
    }
    let mut pids = HashSet::new();
    pids.try_reserve(count)
        .map_err(|_| PathHolderError::BudgetExceeded(SET_ALLOCATION_DETAIL))?;
    for raw in buffer.iter().copied().take(count) {
        let pid = u32::try_from(raw)
            .ok()
            .filter(|pid| *pid != u32::default())
            .ok_or(PathHolderError::Invariant(NONPOSITIVE_PID_DETAIL))?;
        pids.insert(pid);
    }
    Ok(pids)
}

fn reset_errno() {
    unsafe { *__error() = SUCCESS_ERRNO };
}

fn current_errno() -> c_int {
    unsafe { *__error() }
}

unsafe extern "C" {
    fn proc_listpidspath(
        kind: c_uint,
        selector: c_uint,
        path: *const c_char,
        flags: c_uint,
        buffer: *mut c_void,
        bytes: c_int,
    ) -> c_int;
    fn __error() -> *mut c_int;
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_path_holders_live_macos_tests.rs"]
mod live_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_path_holders_macos_tests.rs"]
mod tests;
