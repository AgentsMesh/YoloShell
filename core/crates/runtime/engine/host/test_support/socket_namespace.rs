async fn publish_test_socket(
    namespace: &crate::paths::socket_namespace::SocketNamespace,
    path: &Path,
) -> (
    UnixListener,
    crate::paths::socket_activation::SocketPublication,
) {
    crate::paths::socket_activation::publish(
        namespace.directory().unwrap(),
        path,
        SOCKET_PROBE_TIMEOUT,
        HOST_SOCKET_LIVE_CONTEXT,
    )
    .await
    .unwrap()
}
