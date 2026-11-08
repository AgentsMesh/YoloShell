pub(crate) struct WorkerSocketCleanup {
    _publication: crate::paths::socket_activation::SocketPublication,
    _instance_lock: WorkerInstanceLock,
    _socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
}

impl WorkerSocketCleanup {
    pub(crate) fn capture(
        instance_lock: WorkerInstanceLock,
        publication: crate::paths::socket_activation::SocketPublication,
        socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
    ) -> Self {
        Self {
            _publication: publication,
            _instance_lock: instance_lock,
            _socket_namespace: socket_namespace,
        }
    }
}

pub(crate) async fn bind_worker_socket_retained(
    instance_lock: WorkerInstanceLock,
    namespace: &WorkerNamespace,
    path: &Path,
) -> Result<(UnixListener, WorkerSocketCleanup)> {
    let parent = path.parent().context("worker socket has no parent")?;
    let directory = namespace.lease().directory(parent)?;
    let (listener, publication) = crate::paths::socket_activation::publish(
        directory,
        path,
        WORKER_SOCKET_PROBE_TIMEOUT,
        WORKER_SOCKET_LIVE_CONTEXT,
    )
    .await?;
    let cleanup = WorkerSocketCleanup::capture(
        instance_lock,
        publication,
        namespace.socket_namespace(),
    );
    namespace.verify()?;
    Ok((listener, cleanup))
}

#[cfg(test)]
pub(crate) async fn bind_worker_socket(
    paths: &Paths,
    terminal_id: Uuid,
    path: &Path,
) -> Result<(UnixListener, WorkerSocketCleanup)> {
    let instance_lock = WorkerInstanceLock::acquire(paths, terminal_id)?;
    let namespace = WorkerNamespace::capture(paths, terminal_id)?;
    bind_worker_socket_retained(instance_lock, &namespace, path).await
}

#[cfg(test)]
pub(crate) enum WorkerSocketProbe {
    Live,
    Refused,
    Indeterminate(io::Error),
    TimedOut,
}

#[cfg(test)]
pub(crate) async fn bounded_worker_socket_probe<F, T>(probe: F, timeout: Duration) -> WorkerSocketProbe
where
    F: Future<Output = io::Result<T>>,
{
    match tokio::time::timeout(timeout, probe).await {
        Ok(Ok(_)) => WorkerSocketProbe::Live,
        Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => {
            WorkerSocketProbe::Refused
        }
        Ok(Err(error)) => WorkerSocketProbe::Indeterminate(error),
        Err(_) => WorkerSocketProbe::TimedOut,
    }
}

#[cfg(test)]
pub(crate) fn finish_worker_socket_probe(
    path: &Path,
    identity: WorkerSocketIdentity,
    probe: WorkerSocketProbe,
) -> Result<()> {
    finish_worker_socket_probe_inner(path, identity, probe, || Ok(()))
}

#[cfg(test)]
pub(crate) fn finish_worker_socket_probe_with_hook<F>(
    path: &Path,
    identity: WorkerSocketIdentity,
    probe: WorkerSocketProbe,
    before_remove: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    finish_worker_socket_probe_inner(path, identity, probe, before_remove)
}

#[cfg(test)]
pub(crate) fn finish_worker_socket_probe_inner<F>(
    path: &Path,
    identity: WorkerSocketIdentity,
    probe: WorkerSocketProbe,
    before_remove: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match probe {
        WorkerSocketProbe::Live => bail!(WORKER_SOCKET_LIVE_CONTEXT),
        WorkerSocketProbe::Refused => {
            before_remove()?;
            remove_stale_worker_socket(path, identity)
        }
        WorkerSocketProbe::Indeterminate(error) => {
            Err(error).context(WORKER_SOCKET_INDETERMINATE_CONTEXT)
        }
        WorkerSocketProbe::TimedOut => bail!(WORKER_SOCKET_PROBE_TIMEOUT_CONTEXT),
    }
}

#[cfg(test)]
pub(crate) fn remove_stale_worker_socket(path: &Path, expected: WorkerSocketIdentity) -> Result<()> {
    anyhow::ensure!(
        worker_socket_identity(path).ok() == Some(expected),
        WORKER_SOCKET_CHANGED_CONTEXT
    );
    std::fs::remove_file(path)
        .with_context(|| format!("{WORKER_SOCKET_REMOVE_CONTEXT} {}", path.display()))
}
