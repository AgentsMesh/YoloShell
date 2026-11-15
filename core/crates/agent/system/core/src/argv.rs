use anyhow::{Context, Result, bail, ensure};
#[cfg(target_os = "macos")]
use std::collections::BTreeMap;
#[cfg(any(target_os = "linux", test))]
use std::io::Read;

#[cfg(target_os = "linux")]
pub fn process_argv(pid: u32) -> Result<Vec<String>> {
    let file = std::fs::File::open(yolo_process_identity::procfs::cmdline_path(pid))
        .with_context(|| format!("read argv for process {pid}"))?;
    read_linux_argv(file)
}

#[cfg(any(target_os = "linux", test))]
fn read_linux_argv(reader: impl Read) -> Result<Vec<String>> {
    use super::constants::{
        MAX_SINGLE_PROCESS_ARGV_BYTES, PROCESS_ARGV_READ_SENTINEL_BYTES, process_argv_error,
    };
    let byte_limit = MAX_SINGLE_PROCESS_ARGV_BYTES
        .checked_add(PROCESS_ARGV_READ_SENTINEL_BYTES)
        .context(process_argv_error::RAW_LIMIT)?;
    let read_limit = u64::try_from(byte_limit).context(process_argv_error::RAW_LIMIT)?;
    let mut bytes = Vec::with_capacity(byte_limit);
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .context(process_argv_error::READ)?;
    ensure!(
        bytes.len() <= MAX_SINGLE_PROCESS_ARGV_BYTES,
        process_argv_error::RAW_LIMIT
    );
    ensure!(
        bytes.is_empty() || bytes.last() == Some(&super::constants::NUL_BYTE),
        process_argv_error::UNTERMINATED
    );
    Ok(parse_nul_strings(&bytes, usize::MAX))
}

#[cfg(target_os = "macos")]
pub struct ArgvSnapshot {
    pub values: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub bytes: usize,
}

#[cfg(target_os = "macos")]
pub fn process_argv_bounded(
    pid: u32,
    byte_limit: usize,
    environment_keys: &[&str],
) -> Result<ArgvSnapshot> {
    use super::constants::MAX_PROCESS_ENVIRONMENT_KEYS;
    ensure!(
        environment_keys.len() <= MAX_PROCESS_ENVIRONMENT_KEYS,
        "process environment key limit exceeded"
    );
    ensure!(pid > 0, "process ID must be positive");
    let pid = i32::try_from(pid).context("process ID exceeds c_int")?;
    let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut size = 0usize;
    let status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            std::ptr::null_mut(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    ensure_sysctl_success(status, "measure process argv")?;
    ensure!(size <= byte_limit, "process argv exceeds its scan budget");
    let mut bytes = vec![0u8; size];
    let status = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            bytes.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    ensure_sysctl_success(status, "read process argv")?;
    ensure!(
        size <= bytes.len(),
        "process argv grew beyond its measured buffer"
    );
    bytes.truncate(size);
    let (values, environment) = parse_macos_procargs(&bytes, environment_keys)?;
    Ok(ArgvSnapshot {
        values,
        environment,
        bytes: size,
    })
}

#[cfg(target_os = "macos")]
fn ensure_sysctl_success(status: i32, operation: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(operation.to_owned())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn process_argv(_pid: u32) -> Result<Vec<String>> {
    bail!("structured process argv is unsupported on this platform")
}

#[cfg(target_os = "macos")]
fn parse_macos_procargs(
    bytes: &[u8],
    environment_keys: &[&str],
) -> Result<(Vec<String>, BTreeMap<String, String>)> {
    use super::constants::NUL_BYTE;
    let prefix_len = std::mem::size_of::<i32>();
    let count_bytes = bytes
        .get(..prefix_len)
        .context("process argv omitted argc")?;
    let argc = i32::from_ne_bytes(count_bytes.try_into().unwrap());
    if argc < 0 {
        bail!("process argv reported a negative argc");
    }
    let payload = &bytes[prefix_len..];
    let executable_end = payload
        .iter()
        .position(|byte| *byte == NUL_BYTE)
        .context("process argv omitted executable terminator")?;
    let arguments = payload[executable_end + 1..]
        .iter()
        .position(|byte| *byte != NUL_BYTE)
        .map(|offset| &payload[executable_end + 1 + offset..])
        .unwrap_or_default();
    let (argv, environment) = split_arguments(arguments, argc as usize)?;
    Ok((
        argv,
        super::process_environment::parse_selected_environment(environment, environment_keys)?,
    ))
}

#[cfg(target_os = "macos")]
fn split_arguments(mut bytes: &[u8], count: usize) -> Result<(Vec<String>, &[u8])> {
    use super::constants::NUL_BYTE;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let end = bytes
            .iter()
            .position(|byte| *byte == NUL_BYTE)
            .context("process argv omitted argument terminator")?;
        values.push(String::from_utf8_lossy(&bytes[..end]).into_owned());
        bytes = &bytes[end + 1..];
    }
    Ok((values, bytes))
}

#[cfg(any(target_os = "linux", test))]
fn parse_nul_strings(bytes: &[u8], limit: usize) -> Vec<String> {
    use super::constants::NUL_BYTE;
    bytes
        .split(|byte| *byte == NUL_BYTE)
        .filter(|field| !field.is_empty())
        .take(limit)
        .map(|field| String::from_utf8_lossy(field).into_owned())
        .collect()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/argv_linux_tests.rs"]
mod linux_tests;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/argv_macos_process_tests.rs"]
mod macos_process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/argv_tests.rs"]
mod tests;
