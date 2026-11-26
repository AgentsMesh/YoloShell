impl Host {
    pub(crate) async fn prepare_socket(&self) -> Result<()> {
        let identity = match socket_identity(&self.socket) {
            Ok(identity) => identity,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error).context("inspect existing host socket"),
        };
        let probe =
            bounded_host_socket_probe(UnixStream::connect(&self.socket), SOCKET_PROBE_TIMEOUT)
                .await;
        finish_host_socket_probe(&self.socket, identity, probe)
    }
}

pub(crate) enum HostSocketProbe {
    Live,
    Refused,
    Indeterminate(std::io::Error),
    TimedOut,
}

pub(crate) async fn bounded_host_socket_probe<F, T>(probe: F, timeout: Duration) -> HostSocketProbe
where
    F: Future<Output = std::io::Result<T>>,
{
    match tokio::time::timeout(timeout, probe).await {
        Ok(Ok(_)) => HostSocketProbe::Live,
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            HostSocketProbe::Refused
        }
        Ok(Err(error)) => HostSocketProbe::Indeterminate(error),
        Err(_) => HostSocketProbe::TimedOut,
    }
}

pub(crate) fn finish_host_socket_probe(
    path: &Path,
    identity: SocketIdentity,
    probe: HostSocketProbe,
) -> Result<()> {
    finish_host_socket_probe_inner(path, identity, probe, || Ok(()))
}

#[cfg(test)]
pub(crate) fn finish_host_socket_probe_with_hook<F>(
    path: &Path,
    identity: SocketIdentity,
    probe: HostSocketProbe,
    before_remove: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    finish_host_socket_probe_inner(path, identity, probe, before_remove)
}

pub(crate) fn finish_host_socket_probe_inner<F>(
    path: &Path,
    identity: SocketIdentity,
    probe: HostSocketProbe,
    before_remove: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match probe {
        HostSocketProbe::Live => bail!("{HOST_SOCKET_LIVE_CONTEXT}: {}", path.display()),
        HostSocketProbe::Refused => {
            before_remove()?;
            remove_stale_host_socket(path, identity)
        }
        HostSocketProbe::Indeterminate(error) => {
            Err(error).context(HOST_SOCKET_PROBE_ERROR_CONTEXT)
        }
        HostSocketProbe::TimedOut => bail!(HOST_SOCKET_PROBE_TIMEOUT_CONTEXT),
    }
}

pub(crate) fn remove_stale_host_socket(path: &Path, expected: SocketIdentity) -> Result<()> {
    anyhow::ensure!(
        socket_identity(path).ok() == Some(expected),
        HOST_SOCKET_CHANGED_CONTEXT
    );
    std::fs::remove_file(path)
        .with_context(|| format!("remove stale host socket {}", path.display()))
}
