use anyhow::Result;

#[cfg(target_os = "macos")]
pub fn process_environment_value(pid: u32, key: &str) -> Result<Option<String>> {
    let mut snapshot = super::argv::process_argv_bounded(
        pid,
        super::constants::MAX_SINGLE_PROCESS_ARGV_BYTES,
        &[key],
    )?;
    Ok(snapshot.environment.remove(key))
}

#[cfg(target_os = "linux")]
pub fn process_environment_value(pid: u32, key: &str) -> Result<Option<String>> {
    let file = std::fs::File::open(yolo_process_identity::procfs::environment_path(pid))?;
    let bytes = super::process_environment::read_process_environment(file)?;
    let mut selected = super::process_environment::parse_selected_environment(&bytes, &[key])?;
    Ok(selected.remove(key))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn process_environment_value(_pid: u32, _key: &str) -> Result<Option<String>> {
    anyhow::bail!("process environment inspection is unsupported on this platform")
}
