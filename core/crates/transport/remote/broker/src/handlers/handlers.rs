use super::constants::*;
use super::protocol::*;
use super::runtime_proxy;
use super::server_admission::{BrokerAdmission, permits_pending_request};
use super::sockets::verify_peer;
use super::timing::MANAGEMENT_OPERATION_TIMEOUT;
use super::{BrokerConfig, BrokerConnectionPool};
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Notify;
#[cfg(test)]
pub(super) async fn proxy_runtime(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
) -> Result<()> {
    verify_peer(&local)?;
    runtime_proxy::proxy(local, pool).await
}
#[cfg(test)]
pub(super) async fn proxy_runtime_admitted(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    attachments: Arc<tokio::sync::Semaphore>,
) -> Result<()> {
    verify_peer(&local)?;
    runtime_proxy::proxy_admitted(local, pool, attachments).await
}
pub(super) async fn setup_runtime_admitted(
    local: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    attachments: Arc<tokio::sync::Semaphore>,
) -> Result<Option<runtime_proxy::EstablishedRuntime>> {
    verify_peer(&local)?;
    runtime_proxy::setup_admitted(local, pool, attachments).await
}
#[cfg(test)]
pub(super) async fn manage(
    stream: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    config: Arc<BrokerConfig>,
    shutdown: Arc<Notify>,
    retirement: Arc<Notify>,
    admission: Arc<BrokerAdmission>,
) -> Result<()> {
    manage_upgradable(
        stream,
        pool,
        config,
        shutdown,
        retirement,
        admission,
        Arc::new(tokio::sync::Semaphore::new(
            super::hub_peer_constants::MAX_HUB_TUNNELS,
        )),
    )
    .await
    .map(drop)
}

pub(super) async fn manage_upgradable(
    mut stream: LocalUnixStream,
    pool: Arc<BrokerConnectionPool>,
    config: Arc<BrokerConfig>,
    shutdown: Arc<Notify>,
    retirement: Arc<Notify>,
    admission: Arc<BrokerAdmission>,
    hub_capacity: Arc<tokio::sync::Semaphore>,
) -> Result<Option<super::hub_peer_proxy::EstablishedHubPeer>> {
    verify_peer(&stream)?;
    loop {
        let request: BrokerRequestEnvelope = deadline(
            LOCAL_HANDSHAKE_TIMEOUT,
            crate::protocol::read_frame(&mut stream),
        )
        .await??;
        if !admission.is_open() {
            return Ok(None);
        }
        let request_id = request.request_id;
        if request.protocol_version == BROKER_PROTOCOL_VERSION && admission.is_adopted() {
            if let Some((call, action)) = super::provisioning_rpc::select(&request.request) {
                super::provisioning_rpc::serve(
                    &mut stream,
                    &pool,
                    &admission,
                    request_id,
                    call,
                    action,
                )
                .await?;
                return Ok(None);
            }
            if let BrokerRequest::OpenHubPeer(open) = &request.request {
                return super::hub_peer_proxy::setup(
                    stream,
                    &pool,
                    request_id,
                    open.clone(),
                    hub_capacity,
                )
                .await;
            }
        }
        let outcome = if !admission.is_adopted() && !permits_pending_request(&request.request) {
            DispatchOutcome::response(BrokerResponse::failure(
                request_id,
                BROKER_ERROR_CODE,
                BROKER_ADOPTION_PENDING_MESSAGE,
            ))
        } else {
            match deadline(
                MANAGEMENT_OPERATION_TIMEOUT,
                dispatch::dispatch(&pool, &config, request, admission.is_adopted()),
            )
            .await
            {
                Ok(value) => value,
                Err(error) => DispatchOutcome::response(BrokerResponse::failure(
                    request_id,
                    BROKER_ERROR_CODE,
                    error.to_string(),
                )),
            }
        };
        deadline(
            LOCAL_HANDSHAKE_TIMEOUT,
            crate::protocol::write_frame(&mut stream, &outcome.response),
        )
        .await??;
        match outcome.control {
            BrokerControl::Continue => {}
            BrokerControl::Rotate => {
                shutdown.notify_one();
                return Ok(None);
            }
            BrokerControl::Retire => {
                retirement.notify_one();
                return Ok(None);
            }
        }
    }
}
pub(super) fn close_saturated<T>(stream: T) {
    drop(stream);
}
async fn deadline<F, T>(duration: std::time::Duration, future: F) -> Result<T>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| anyhow::anyhow!(LOCAL_PROTOCOL_TIMEOUT_MESSAGE))
}
#[path = "handlers_dispatch.rs"]
mod dispatch;
struct DispatchOutcome {
    response: BrokerResponse,
    control: BrokerControl,
}
#[derive(Clone, Copy)]
enum BrokerControl {
    Continue,
    Rotate,
    Retire,
}
impl DispatchOutcome {
    fn response(response: BrokerResponse) -> Self {
        Self {
            response,
            control: BrokerControl::Continue,
        }
    }
}
include!("handlers_shutdown.rs");
