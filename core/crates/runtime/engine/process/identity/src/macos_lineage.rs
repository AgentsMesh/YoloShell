use crate::macos_kernel::ProcUniqueIdentifierInfo;
use anyhow::{Context, Result, ensure};
use std::mem::{MaybeUninit, size_of};

const UNIQUE_IDENTIFIER_FLAVOR: libc::c_int = 17;
const QUERY_ARGUMENT: u64 = 0;
const QUERY_CONTEXT: &str = "capture macOS controller lineage";
const RESPONSE_SIZE_DETAIL: &str = "controller lineage response has the wrong size";
const INVALID_IDENTITY_DETAIL: &str = "controller lineage identity is invalid";
const CHANGED_IDENTITY_DETAIL: &str = "controller lineage changed during capture";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessLineage {
    pub pid: u32,
    pub parent_pid: u32,
    pub unique_id: u64,
    pub parent_unique_id: u64,
}

pub fn capture(pid: u32) -> Result<Option<ProcessLineage>> {
    capture_with(
        pid,
        || query::<libc::proc_bsdshortinfo>(pid, libc::PROC_PIDT_SHORTBSDINFO),
        || query::<ProcUniqueIdentifierInfo>(pid, UNIQUE_IDENTIFIER_FLAVOR),
    )
    .with_context(|| format!("{QUERY_CONTEXT}: pid={pid}"))
}

fn capture_with(
    pid: u32,
    mut short: impl FnMut() -> Result<Option<libc::proc_bsdshortinfo>>,
    mut unique: impl FnMut() -> Result<Option<ProcUniqueIdentifierInfo>>,
) -> Result<Option<ProcessLineage>> {
    let Some(before) = unique()? else {
        return Ok(None);
    };
    let Some(first) = short()? else {
        return Ok(None);
    };
    let Some(middle) = unique()? else {
        return Ok(None);
    };
    let Some(second) = short()? else {
        return Ok(None);
    };
    let Some(after) = unique()? else {
        return Ok(None);
    };
    ensure!(
        first.pbsi_pid == pid && second.pbsi_pid == pid && before.unique_id != 0,
        INVALID_IDENTITY_DETAIL
    );
    ensure!(
        first.pbsi_ppid == second.pbsi_ppid
            && same_lineage(&before, &middle)
            && same_lineage(&middle, &after),
        CHANGED_IDENTITY_DETAIL
    );
    ensure!(
        first.pbsi_ppid == 0 || before.parent_unique_id != 0,
        INVALID_IDENTITY_DETAIL
    );
    if first.pbsi_status == libc::SZOMB || second.pbsi_status == libc::SZOMB {
        return Ok(None);
    }
    Ok(Some(ProcessLineage {
        pid,
        parent_pid: first.pbsi_ppid,
        unique_id: before.unique_id,
        parent_unique_id: before.parent_unique_id,
    }))
}

fn same_lineage(first: &ProcUniqueIdentifierInfo, second: &ProcUniqueIdentifierInfo) -> bool {
    first.unique_id == second.unique_id && first.parent_unique_id == second.parent_unique_id
}

fn query<T>(pid: u32, flavor: libc::c_int) -> Result<Option<T>> {
    let pid = libc::c_int::try_from(pid).context(INVALID_IDENTITY_DETAIL)?;
    ensure!(pid > 0, INVALID_IDENTITY_DETAIL);
    let mut response = MaybeUninit::<T>::uninit();
    let expected = libc::c_int::try_from(size_of::<T>()).context(RESPONSE_SIZE_DETAIL)?;
    // Both private call sites use integer-only libproc structures with fixed C layouts.
    let returned = unsafe {
        *libc::__error() = 0;
        libc::proc_pidinfo(
            pid,
            flavor,
            QUERY_ARGUMENT,
            response.as_mut_ptr().cast(),
            expected,
        )
    };
    if returned <= 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        return Err(error).context(QUERY_CONTEXT);
    }
    ensure!(returned == expected, RESPONSE_SIZE_DETAIL);
    Ok(Some(unsafe { response.assume_init() }))
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/macos_lineage_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/macos_lineage_tests.rs"]
mod integration_tests;
