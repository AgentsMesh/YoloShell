use super::*;

pub(super) fn command_basename(command: &str) -> String {
    Path::new(command)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(command)
        .to_owned()
}

pub(super) fn resolve_cwd(input: &Path, host_home: Option<&Path>) -> Result<PathBuf> {
    let text = input
        .to_str()
        .context("working directory must be valid UTF-8")?;
    let expanded = if text == "~" {
        host_home
            .context("cannot expand ~ because the host HOME is unavailable")?
            .to_path_buf()
    } else if let Some(remainder) = text.strip_prefix("~/") {
        host_home
            .context("cannot expand ~/ because the host HOME is unavailable")?
            .join(remainder)
    } else if text.starts_with('~') {
        bail!("~user working directories are not supported; use ~ or an absolute path");
    } else {
        input.to_path_buf()
    };
    anyhow::ensure!(
        expanded.is_absolute(),
        "working directory must be absolute or begin with ~/"
    );
    let canonical = expanded
        .canonicalize()
        .with_context(|| format!("working directory does not exist: {}", expanded.display()))?;
    anyhow::ensure!(
        canonical.is_dir(),
        "working directory is not a directory: {}",
        canonical.display()
    );
    Ok(canonical)
}

impl Host {
    pub(super) fn unreachable_view(&self, record: &RuntimeRecord) -> RuntimeView {
        let exit = self
            .read_namespace_json(&self.paths.exit_record(record.terminal_id))
            .ok();
        let mut view = unreachable_view_with_exit(record, exit, &self.boot_id);
        view.presentation =
            crate::worker::load_presentation(&self.paths, record.terminal_id, record.runtime_epoch);
        #[cfg(target_os = "linux")]
        {
            view.capabilities = (view.status != TerminalRuntimeStatus::Lost)
                .then(|| self.linux_fallback_capabilities(record))
                .flatten();
        }
        view
    }
}

#[cfg(test)]
pub(super) fn unreachable_view(paths: &Paths, record: &RuntimeRecord) -> RuntimeView {
    unreachable_view_with_exit(
        record,
        crate::worker::load_exit(paths, record.terminal_id),
        &process::boot_id(),
    )
}

pub(super) fn capabilities() -> Vec<String> {
    use crate::protocol::capability;
    let mut values = [
        capability::AGENT_RESUME,
        capability::CAPNP_BINARY,
        capability::CREATION_CAPABILITY,
        capability::DURABLE_PTY,
        capability::HOST_ADOPTION,
        capability::HOST_BUILD,
        capability::IDLE_HOST_SHUTDOWN,
        capability::OUTPUT_JOURNAL,
        capability::SSH_STDIO_BRIDGE,
        capability::WORKSPACE_INSPECT,
        capability::WORKTREE,
        capability::WRITER_LEASE_V2,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    values.extend(yolo_host_provider_capabilities::provider_capabilities());
    values.push(yolo_agent_contract::PROVIDER_CATALOG_CAPABILITY.to_owned());
    values.sort();
    values.dedup();
    values
}

pub(super) async fn wait_for_process_identity_exit(
    pid: u32,
    start_time: u64,
    timeout: Duration,
) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    while !process_identity_is_gone(pid, start_time) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(PROCESS_EXIT_POLL_INTERVAL).await;
    }
    process_identity_is_gone(pid, start_time)
}

pub(super) async fn wait_for_runtime_identities_exit(
    record: &RuntimeRecord,
    timeout: Duration,
) -> bool {
    let worker =
        wait_for_process_identity_exit(record.worker_pid, record.worker_start_time, timeout);
    let child = wait_for_process_identity_exit(record.child_pid, record.child_start_time, timeout);
    let (worker_gone, child_gone) = tokio::join!(worker, child);
    worker_gone && child_gone
}

pub(super) async fn socket_may_be_serving(path: &Path) -> bool {
    !matches!(
        tokio::time::timeout(SOCKET_PROBE_TIMEOUT, UnixStream::connect(path)).await,
        Ok(Err(_))
    )
}
