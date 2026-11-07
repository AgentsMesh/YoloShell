async fn publish(
    directory: crate::paths::directory::PrivateDirectory,
    path: &Path,
) -> Result<(
    UnixListener,
    crate::paths::socket_activation::SocketPublication,
)> {
    crate::paths::socket_activation::publish(
        directory,
        path,
        CODEX_SOCKET_PROBE_TIMEOUT,
        CODEX_SOCKET_LIVE_ERROR,
    )
    .await
    .context(CODEX_SOCKET_BIND_ERROR)
}

async fn publish_after_recovery<F>(
    directory: crate::paths::directory::PrivateDirectory,
    path: &Path,
    recovery: F,
) -> Result<(
    UnixListener,
    crate::paths::socket_activation::SocketPublication,
)>
where
    F: FnOnce() -> Result<()>,
{
    crate::paths::socket_activation::publish_after(
        directory,
        path,
        CODEX_SOCKET_PROBE_TIMEOUT,
        CODEX_SOCKET_LIVE_ERROR,
        recovery,
    )
    .await
    .context(CODEX_SOCKET_BIND_ERROR)
}

fn socket_directory(
    namespace: &WorkerNamespace,
    path: &Path,
) -> Result<crate::paths::directory::PrivateDirectory> {
    let parent = path.parent().context(CODEX_SOCKET_PARENT_ERROR)?;
    namespace.lease().directory(parent)
}

fn retain_socket_directory(
    directory: crate::paths::directory::PrivateDirectory,
) -> Result<(
    crate::paths::directory::PrivateDirectory,
    crate::paths::directory::PrivateDirectory,
)> {
    verify_socket_directory(&directory)?;
    let publication_directory = directory.try_clone()?;
    Ok((directory, publication_directory))
}

fn verify_socket_directory(directory: &crate::paths::directory::PrivateDirectory) -> Result<()> {
    directory
        .verify_path()
        .context(CODEX_SOCKET_NAMESPACE_ERROR)
}
