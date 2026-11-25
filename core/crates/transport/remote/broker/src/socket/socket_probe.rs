use super::constants::BROKER_SOCKET_PROBE_TIMEOUT;
use std::future::Future;
use std::path::Path;

pub(super) enum BrokerSocketProbe {
    Live,
    Refused,
    Indeterminate(std::io::Error),
    TimedOut,
}

pub(super) async fn probe_broker_socket(path: &Path) -> BrokerSocketProbe {
    bounded_broker_socket_probe(
        tokio::net::UnixStream::connect(path),
        BROKER_SOCKET_PROBE_TIMEOUT,
    )
    .await
}

pub(super) async fn bounded_broker_socket_probe<F, T>(
    probe: F,
    timeout: std::time::Duration,
) -> BrokerSocketProbe
where
    F: Future<Output = std::io::Result<T>>,
{
    match tokio::time::timeout(timeout, probe).await {
        Ok(Ok(_)) => BrokerSocketProbe::Live,
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            BrokerSocketProbe::Refused
        }
        Ok(Err(error)) => BrokerSocketProbe::Indeterminate(error),
        Err(_) => BrokerSocketProbe::TimedOut,
    }
}
