use super::{
    BrokerConnectionPool, BrokerResponse, BrokerResult,
    constants::*,
    runtime_proxy::{Stage, monitor},
};
use crate::local_unix::LocalUnixStream;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;
use yolo_broker_hub_protocol::{BrokerHubError, BrokerHubOpenRequest, BrokerHubOpened};

pub(super) struct EstablishedHubPeer {
    local: LocalUnixStream,
    remote: crate::remote::HubPeerStream,
    prefix: Vec<u8>,
    _permit: OwnedSemaphorePermit,
}

impl EstablishedHubPeer {
    pub(super) async fn forward(self) -> Result<()> {
        let _permit = self._permit;
        super::runtime_forward::forward(self.local, Box::new(self.remote), self.prefix).await
    }
}

pub(super) async fn setup(
    mut local: LocalUnixStream,
    pool: &BrokerConnectionPool,
    request_id: Uuid,
    request: BrokerHubOpenRequest,
    capacity: Arc<Semaphore>,
) -> Result<Option<EstablishedHubPeer>> {
    let permit = match capacity.try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            reject(&mut local, request_id, BrokerHubError::Capacity.into()).await?;
            return Ok(None);
        }
    };
    let mut prefix = Vec::new();
    let opened = {
        let opening = tokio::time::timeout(LOCAL_HANDSHAKE_TIMEOUT, pool.open_hub_peer(&request));
        tokio::pin!(opening);
        match monitor(&mut local, &mut prefix, opening.as_mut()).await? {
            Stage::LocalEof => return Ok(None),
            Stage::Complete(Ok(Ok(opened))) => opened,
            Stage::Complete(result) => {
                let error = match result {
                    Ok(Err(error)) => error,
                    Err(error) => error.into(),
                    Ok(Ok(_)) => unreachable!(),
                };
                reject(&mut local, request_id, error).await?;
                return Ok(None);
            }
        }
    };
    let response = BrokerResponse::success(
        request_id,
        BrokerResult::HubPeer(BrokerHubOpened {
            request,
            channel_id: opened.identity.channel_id,
        }),
    );
    tokio::time::timeout(
        LOCAL_HANDSHAKE_TIMEOUT,
        crate::protocol::write_frame(&mut local, &response),
    )
    .await??;
    Ok(Some(EstablishedHubPeer {
        local,
        remote: opened.stream,
        prefix,
        _permit: permit,
    }))
}

async fn reject(local: &mut LocalUnixStream, request: Uuid, error: anyhow::Error) -> Result<()> {
    let response = super::hub_peer_response::failure(request, &error);
    tokio::time::timeout(
        LOCAL_HANDSHAKE_TIMEOUT,
        crate::protocol::write_frame(local, &response),
    )
    .await??;
    Ok(())
}
