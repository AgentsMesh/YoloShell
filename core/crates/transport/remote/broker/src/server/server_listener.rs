async fn serve_listener<F, S>(
    listener: &UnixListener,
    namespace: Arc<BrokerNamespace>,
    label: &'static str,
    mut on_stream: F,
) -> Result<std::convert::Infallible>
where
    F: FnMut(S),
    S: From<UnixStream>,
{
    let mut backoff = crate::ingress::AcceptBackoff::new(
        ACCEPT_FAILURE_INITIAL_BACKOFF,
        ACCEPT_FAILURE_MAX_BACKOFF,
    );
    loop {
        namespace.verify()?;
        let accepted = listener.accept().await.map(|(stream, _)| S::from(stream));
        namespace.verify()?;
        handle_listener_result(accepted, &mut backoff, label, &mut on_stream).await;
    }
}

pub(super) async fn handle_listener_result<F, S>(
    accepted: io::Result<S>,
    backoff: &mut crate::ingress::AcceptBackoff,
    label: &'static str,
    on_stream: &mut F,
) where
    F: FnMut(S),
{
    match accepted {
        Ok(stream) => {
            backoff.reset();
            on_stream(stream);
        }
        Err(error) => {
            let delay = backoff.next_delay();
            tracing::warn!(%error, ?delay, listener = label, "broker accept failed; retrying");
            tokio::time::sleep(delay).await;
        }
    }
}
