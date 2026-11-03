use super::*;

pub(super) fn exchange(
    stream: std::os::unix::net::UnixStream,
    request: RequestEnvelope,
) -> Option<ResponseEnvelope> {
    // Fixture cleanup can run from Drop inside another Tokio runtime.
    // Keep the bounded RPC reactor on its own scoped thread.
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .ok()?;
                runtime.block_on(async move {
                    stream.set_nonblocking(true).ok()?;
                    let stream = tokio::net::UnixStream::from_std(stream).ok()?;
                    let deadline = tokio::time::Instant::now() + CLEANUP_RPC_TIMEOUT;
                    let mut channel = yolo_runtime_rpc::RuntimeConnection::open(stream, deadline)
                        .await
                        .ok()?;
                    channel.exchange(&request, deadline).await.ok()
                })
            })
            .join()
            .ok()
            .flatten()
    })
}
