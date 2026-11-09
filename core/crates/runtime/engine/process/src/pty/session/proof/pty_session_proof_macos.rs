use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use crate::process::constants::{
    MACOS_PROCESS_ARGUMENT_FIELD_SEPARATOR_BYTES, MACOS_PROCESS_ARGUMENT_POPULATED_EXTENT_BYTES,
    MACOS_SYSCTL_SUCCESS,
};
use anyhow::{Context, Result, anyhow, ensure};
use std::mem::{MaybeUninit, size_of};

const ARGMAX_READ_CONTEXT: &str = "read maximum PTY proof process argument size";
const ARGMAX_SIZE_DETAIL: &str = "maximum PTY proof process argument size changed shape";
const ARGMAX_VALUE_DETAIL: &str = "maximum PTY proof process argument size is invalid";
const ARGV_READ_CONTEXT: &str = "read PTY proof process environment";
const ARGV_GROWTH_DETAIL: &str = "PTY proof process environment grew during capture";
const ARGV_PREFIX_DETAIL: &str = "PTY proof process environment omitted argc";
const ARGV_COUNT_DETAIL: &str = "PTY proof process environment reported a negative argc";
const ARGV_EXECUTABLE_DETAIL: &str = "PTY proof process environment omitted executable";
const ARGV_ARGUMENT_DETAIL: &str = "PTY proof process environment omitted an argument";
const ARGV_ENVIRONMENT_DETAIL: &str = "PTY proof process environment was not disclosed";
const PID_RANGE_CONTEXT: &str = "PTY proof process id exceeds platform range";
const ENVIRONMENT_ASSIGNMENT: u8 = b'=';
const FIELD_TERMINATOR: u8 = 0;

pub(super) struct EnvironmentCapability {
    pub(super) matches: bool,
}

pub(super) fn environment_capability(
    pid: u32,
    environment_name: &str,
    capability: &str,
    budget: &mut CapabilityScanBudget,
) -> CapabilityProofResult<EnvironmentCapability> {
    let pid = checked_pid(pid).map_err(CapabilityProofError::invariant)?;
    let capacity = match budget.cached_environment_capacity() {
        Some(capacity) => capacity,
        None => process_argument_capacity().map_err(CapabilityProofError::candidate)?,
    };
    let (result, populated) = {
        let bytes = budget.environment_capture_buffer(capacity)?;
        let size = capture_process_arguments(pid, bytes)?;
        // SAFETY: a successful sysctl initialized the reported byte extent.
        let bytes = unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), size) };
        let populated = populated_argument_extent(bytes);
        (
            capability_from_arguments(&bytes[..populated], environment_name, capability),
            populated,
        )
    };
    budget.admit_environment(populated)?;
    result
}

fn capability_from_arguments(
    bytes: &[u8],
    environment_name: &str,
    capability: &str,
) -> CapabilityProofResult<EnvironmentCapability> {
    let environment = environment_fields(bytes).map_err(CapabilityProofError::invariant)?;
    if !contains_environment_field(environment) {
        return Err(CapabilityProofError::candidate(anyhow!(
            ARGV_ENVIRONMENT_DETAIL
        )));
    }
    let expected = format!("{environment_name}={capability}");
    let matches = environment
        .split(|byte| *byte == FIELD_TERMINATOR)
        .any(|field| field == expected.as_bytes());
    Ok(EnvironmentCapability { matches })
}

fn capture_process_arguments(
    pid: i32,
    bytes: &mut [MaybeUninit<u8>],
) -> CapabilityProofResult<usize> {
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut size = bytes.len();
    let status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as _,
            bytes.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    ensure_sysctl(status, ARGV_READ_CONTEXT).map_err(CapabilityProofError::candidate)?;
    if size > bytes.len() {
        return Err(CapabilityProofError::invariant(anyhow!(ARGV_GROWTH_DETAIL)));
    }
    Ok(size)
}

fn process_argument_capacity() -> Result<usize> {
    let mut mib = [libc::CTL_KERN, libc::KERN_ARGMAX];
    let mut maximum = libc::c_int::default();
    let mut size = size_of::<libc::c_int>();
    ensure_sysctl(
        unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as _,
                std::ptr::addr_of_mut!(maximum).cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        },
        ARGMAX_READ_CONTEXT,
    )?;
    ensure!(size == size_of::<libc::c_int>(), ARGMAX_SIZE_DETAIL);
    usize::try_from(maximum)
        .ok()
        .filter(|value| *value > usize::default())
        .context(ARGMAX_VALUE_DETAIL)
}

fn populated_argument_extent(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|byte| *byte != FIELD_TERMINATOR)
        .map(|index| {
            index
                .saturating_add(MACOS_PROCESS_ARGUMENT_POPULATED_EXTENT_BYTES)
                .min(bytes.len())
        })
        .unwrap_or(bytes.len().min(size_of::<i32>()))
}

fn contains_environment_field(bytes: &[u8]) -> bool {
    bytes.split(|byte| *byte == FIELD_TERMINATOR).any(|field| {
        field
            .iter()
            .position(|byte| *byte == ENVIRONMENT_ASSIGNMENT)
            .is_some_and(|assignment| assignment > usize::default())
    })
}

fn environment_fields(bytes: &[u8]) -> Result<&[u8]> {
    let prefix = size_of::<i32>();
    let count = i32::from_ne_bytes(
        bytes
            .get(..prefix)
            .context(ARGV_PREFIX_DETAIL)?
            .try_into()?,
    );
    ensure!(count >= 0, ARGV_COUNT_DETAIL);
    let payload = &bytes[prefix..];
    let executable = payload
        .iter()
        .position(|byte| *byte == FIELD_TERMINATOR)
        .context(ARGV_EXECUTABLE_DETAIL)?;
    let offset = payload[executable + MACOS_PROCESS_ARGUMENT_FIELD_SEPARATOR_BYTES..]
        .iter()
        .position(|byte| *byte != FIELD_TERMINATOR)
        .map(|value| executable + MACOS_PROCESS_ARGUMENT_FIELD_SEPARATOR_BYTES + value)
        .unwrap_or(payload.len());
    let mut fields = &payload[offset..];
    for _ in 0..count {
        let end = fields
            .iter()
            .position(|byte| *byte == FIELD_TERMINATOR)
            .context(ARGV_ARGUMENT_DETAIL)?;
        fields = &fields[end + MACOS_PROCESS_ARGUMENT_FIELD_SEPARATOR_BYTES..];
    }
    Ok(fields)
}

fn checked_pid(pid: u32) -> Result<i32> {
    i32::try_from(pid).context(PID_RANGE_CONTEXT)
}

fn ensure_sysctl(status: i32, context: &'static str) -> Result<()> {
    (status == MACOS_SYSCTL_SUCCESS)
        .then_some(())
        .ok_or_else(std::io::Error::last_os_error)
        .context(context)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/proof/pty_session_proof_macos_process_tests.rs"]
mod process_tests;
