use super::super::constants::{MACOS_PIPE_INFO_FLAVOR, MAX_PTY_PROOF_FILE_DESCRIPTORS};
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_fds::list_descriptors;
use anyhow::anyhow;
use std::mem::{MaybeUninit, size_of};

const PIPE_INFO_CONTEXT: &str = "read process capability pipe descriptor";
const PIPE_INFO_SHORT_DETAIL: &str = "short process capability pipe response";
const DESCRIPTOR_CHANGED_DETAIL: &str =
    "process capability descriptor snapshot changed while proving absence";
const EMPTY_QUERY_RESULT: i32 = 0;
const SINGLE_EXACT_CANDIDATE: usize = 1;

#[repr(C)]
struct ProcessFileInfo {
    open_flags: u32,
    status: u32,
    offset: libc::off_t,
    file_type: i32,
    guard_flags: u32,
}

#[repr(C)]
struct PipeInfo {
    stat: libc::vinfo_stat,
    handle: u64,
    peer_handle: u64,
    status: i32,
    reserved: i32,
}

#[repr(C)]
struct PipeFdInfo {
    file: ProcessFileInfo,
    pipe: PipeInfo,
}

pub(in crate::process) struct InheritedPipeEndpoint {
    pub(in crate::process) handle: u64,
    pub(in crate::process) peer_handle: u64,
}

#[derive(Debug, Eq, PartialEq)]
struct PipeFingerprint {
    descriptor: i32,
    handle: u64,
}

enum DescriptorPass {
    Matched,
    Absent(Vec<PipeFingerprint>),
}

pub(super) fn has_stable_descriptor_capability(
    pid: u32,
    expected: u64,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<bool> {
    let first = match pass(pid, expected, budget)? {
        DescriptorPass::Matched => return Ok(true),
        DescriptorPass::Absent(value) => value,
    };
    let second = match pass(pid, expected, budget)? {
        DescriptorPass::Matched => return Ok(true),
        DescriptorPass::Absent(value) => value,
    };
    if first != second {
        return Err(CapabilityProofError::candidate(anyhow!(
            DESCRIPTOR_CHANGED_DETAIL
        )));
    }
    Ok(false)
}

pub(super) fn observes_descriptor_capability(
    pid: u32,
    expected: u64,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<bool> {
    let descriptors = admit_pipe_descriptor_work(
        list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS)?,
        budget,
    )?;
    for descriptor in descriptors {
        if pipe_handle(pid, descriptor.proc_fd)? == expected {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(in crate::process) fn exact_descriptor_holder(pid: u32, expected: u64) -> anyhow::Result<bool> {
    let mut budget = CapabilityScanBudget::start(SINGLE_EXACT_CANDIDATE)?;
    Ok(has_stable_descriptor_capability(
        pid,
        expected,
        &mut budget,
    )?)
}

fn pass(
    pid: u32,
    expected: u64,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<DescriptorPass> {
    let descriptors = admit_pipe_descriptor_work(
        list_descriptors(pid, MAX_PTY_PROOF_FILE_DESCRIPTORS)?,
        budget,
    )?;
    let mut fingerprint = Vec::new();
    for descriptor in descriptors {
        let handle = pipe_handle(pid, descriptor.proc_fd)?;
        if handle == expected {
            return Ok(DescriptorPass::Matched);
        }
        fingerprint.push(PipeFingerprint {
            descriptor: descriptor.proc_fd,
            handle,
        });
    }
    fingerprint.sort_unstable_by_key(|value| (value.descriptor, value.handle));
    Ok(DescriptorPass::Absent(fingerprint))
}

fn admit_pipe_descriptor_work(
    mut descriptors: Vec<libc::proc_fdinfo>,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<Vec<libc::proc_fdinfo>> {
    let pipe_type = libc::PROX_FDTYPE_PIPE as u32;
    descriptors.retain(|descriptor| descriptor.proc_fdtype == pipe_type);
    budget.admit_descriptors(descriptors.len())?;
    Ok(descriptors)
}

pub(in crate::process) fn inherited_pipe_endpoint(
    pid: u32,
    descriptor: i32,
) -> anyhow::Result<InheritedPipeEndpoint> {
    let pipe = pipe_info(pid, descriptor).map_err(anyhow::Error::new)?;
    Ok(InheritedPipeEndpoint {
        handle: pipe.handle,
        peer_handle: pipe.peer_handle,
    })
}

fn pipe_handle(pid: u32, descriptor: i32) -> CapabilityProofResult<u64> {
    Ok(pipe_info(pid, descriptor)?.handle)
}

fn pipe_info(pid: u32, descriptor: i32) -> CapabilityProofResult<PipeInfo> {
    let mut value = MaybeUninit::<PipeFdInfo>::uninit();
    let expected =
        i32::try_from(size_of::<PipeFdInfo>()).map_err(CapabilityProofError::invariant)?;
    unsafe { *libc::__error() = i32::default() };
    let read = unsafe {
        libc::proc_pidfdinfo(
            i32::try_from(pid).map_err(CapabilityProofError::invariant)?,
            descriptor,
            MACOS_PIPE_INFO_FLAVOR,
            value.as_mut_ptr().cast(),
            expected,
        )
    };
    if read <= EMPTY_QUERY_RESULT {
        return Err(CapabilityProofError::candidate(
            anyhow::Error::new(std::io::Error::last_os_error()).context(PIPE_INFO_CONTEXT),
        ));
    }
    if read != expected {
        return Err(CapabilityProofError::invariant(anyhow!(
            PIPE_INFO_SHORT_DETAIL
        )));
    }
    Ok(unsafe { value.assume_init() }.pipe)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/descriptor/pty_session_descriptor_capability_budget_tests.rs"]
mod budget_tests;
