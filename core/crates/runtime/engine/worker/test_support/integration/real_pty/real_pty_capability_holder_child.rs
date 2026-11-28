fn run_fixture_pty_capability_writer_holder() -> Result<()> {
    use std::os::fd::FromRawFd as _;
    let descriptor = std::env::var(FIXTURE_CAPABILITY_HOLDER_DESCRIPTOR_ENV)
        .context(FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL)?
        .parse::<libc::c_int>()
        .context(FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL)?;
    anyhow::ensure!(
        descriptor > libc::STDERR_FILENO,
        FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL
    );
    let owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor) };
    let writer = process::MacosProcessCapabilityChildWriter::from_transferred(owned)?;
    let ready = required_holder_path(FIXTURE_CAPABILITY_HOLDER_READY_ENV)?;
    let release = required_holder_path(FIXTURE_CAPABILITY_HOLDER_RELEASE_ENV)?;
    let parent = process::ProcessIdentity {
        pid: required_holder_number(FIXTURE_CAPABILITY_HOLDER_PARENT_PID_ENV)?,
        start_time: required_holder_number(FIXTURE_CAPABILITY_HOLDER_PARENT_START_ENV)?,
    };
    let direct_parent = u32::try_from(unsafe { libc::getppid() })
        .context(FIXTURE_CAPABILITY_HOLDER_IDENTITY_DETAIL)?;
    anyhow::ensure!(
        direct_parent == parent.pid && parent.matches(),
        FIXTURE_CAPABILITY_HOLDER_IDENTITY_DETAIL
    );
    std::fs::write(ready, EMPTY_MARKER)?;
    while parent.matches() && !release.try_exists()? {
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
    drop(writer);
    Ok(())
}

fn required_holder_path(name: &str) -> Result<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .context(FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL)
}

fn required_holder_number<T>(name: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(name)?
        .parse()
        .context(FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL)
}
