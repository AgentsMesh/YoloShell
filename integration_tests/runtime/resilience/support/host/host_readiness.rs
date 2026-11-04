use super::*;

const HOST_EARLY_EXIT_DETAIL: &str = "host exited before ready";
const HOST_STATUS_INSPECTION_DETAIL: &str = "inspect test Host status";
const HOST_READY_EXIT_DETAIL: &str = "host exited while readiness expired";
const HOST_NOT_READY_DETAIL: &str = "host did not become ready";
const HOST_READY_WAIT_DETAIL: &str = "inspect Host after readiness expired";
const HOST_SOCKET_RESOLUTION_DETAIL: &str = "resolve test Host socket namespace";

pub(crate) async fn wait_for_host(home: &Path, host: &mut HostGuard) {
    let _ = wait_for_host_status(home, host).await;
}

pub(crate) async fn wait_for_host_status(
    home: &Path,
    host: &mut HostGuard,
) -> yolo_runtime::model::HostStatus {
    match wait_for_host_status_result(home, host).await {
        Ok(status) => status,
        Err(error) => panic!("{error}"),
    }
}

pub(crate) async fn wait_for_host_result(home: &Path, host: &mut HostGuard) -> Result<(), String> {
    wait_for_host_status_result(home, host).await.map(drop)
}

async fn wait_for_host_status_result(
    home: &Path,
    host: &mut HostGuard,
) -> Result<yolo_runtime::model::HostStatus, String> {
    let deadline = tokio::time::Instant::now() + HOST_READY_TIMEOUT;
    loop {
        match host.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "{HOST_EARLY_EXIT_DETAIL}: {status}; stderr tail: {}",
                    host.stderr_tail()
                ));
            }
            Err(error) => return Err(format!("{HOST_STATUS_INSPECTION_DETAIL}: {error}")),
            Ok(None) => {}
        }
        let socket = match Paths::new(home).host_socket() {
            Ok(socket) => socket,
            Err(error) => {
                let missing = error.chain().any(|cause| {
                    cause
                        .downcast_ref::<std::io::Error>()
                        .is_some_and(|cause| cause.kind() == std::io::ErrorKind::NotFound)
                });
                if !missing || tokio::time::Instant::now() >= deadline {
                    return Err(host_ready_failure(
                        host,
                        format!("{HOST_SOCKET_RESOLUTION_DETAIL}: {error:#}"),
                    ));
                }
                wait_for_next_host_probe(deadline).await;
                continue;
            }
        };
        let client = Client::new(socket);
        match tokio::time::timeout_at(deadline, client.status()).await {
            Ok(Ok(status)) => match host.verify_ready_status(&status) {
                Ok(()) => return Ok(status),
                Err(error) if tokio::time::Instant::now() >= deadline => {
                    return Err(host_ready_failure(host, format!("{error:#}")));
                }
                Err(_) => {}
            },
            Ok(Err(error)) if tokio::time::Instant::now() >= deadline => {
                return Err(host_ready_failure(host, format!("{error:?}")));
            }
            Ok(Err(_)) => {}
            Err(error) => return Err(host_ready_failure(host, format!("{error:?}"))),
        }
        wait_for_next_host_probe(deadline).await;
    }
}

async fn wait_for_next_host_probe(deadline: tokio::time::Instant) {
    let next_poll = tokio::time::Instant::now() + HOST_READY_POLL_INTERVAL;
    tokio::time::sleep_until(std::cmp::min(next_poll, deadline)).await;
}

fn host_ready_failure(host: &mut HostGuard, error: String) -> String {
    match host.try_wait() {
        Ok(Some(status)) => format!(
            "{HOST_READY_EXIT_DETAIL}: {status}; probe error: {error}; stderr tail: {}",
            host.stderr_tail()
        ),
        Ok(None) => format!(
            "{HOST_NOT_READY_DETAIL}: {error}; stderr tail: {}",
            host.stderr_tail()
        ),
        Err(wait_error) => format!(
            "{HOST_READY_WAIT_DETAIL}: {wait_error}; probe error: {error}; stderr tail: {}",
            host.stderr_tail()
        ),
    }
}
