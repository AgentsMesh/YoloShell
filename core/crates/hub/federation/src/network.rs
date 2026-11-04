use crate::{Federation, constants::*};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{Semaphore, watch},
    task::JoinSet,
    time::{Instant, interval},
};
use yolo_hub_federation_settings::{MAX_PEERS, Peer};
use yolo_hub_peer_diagnostics::{
    INBOUND_PEER_REJECTED, InboundRejections, OutboundFailures, PEER_ATTEMPT_FAILED,
    PEER_RECOVERED, PeerAttempt,
};
use yolo_hub_peer_transport::PeerPins;

impl Federation {
    pub(crate) async fn dial_loop(
        self: Arc<Self>,
        peer: Peer,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let address = peer.dial.as_ref().expect(INVALID_PEER);
        let mut tick = interval(POLL_INTERVAL);
        let mut failures = OutboundFailures::default();
        loop {
            tokio::select! { _ = stop.changed() => break, _ = tick.tick() => {} }
            if self.is_closing() {
                break;
            }
            if self.current(peer.identity.hub()).await.is_some() {
                continue;
            }
            let deadline = Instant::now() + CONNECT_TIMEOUT;
            let attempt = async {
                let transport = yolo_hub_peer_dialer::dial_peer(
                    self.dialer.as_ref(),
                    peer.identity,
                    address,
                    deadline,
                )
                .await
                .map_err(|error| (PeerAttempt::Dial, error))?;
                self.require_open()
                    .map_err(|error| (PeerAttempt::Handshake, error))?;
                let (connection, incoming) = yolo_hub_peer_transport::connect_io(
                    transport,
                    self.key.clone(),
                    peer.identity,
                    deadline,
                )
                .await
                .map_err(|error| (PeerAttempt::Handshake, error))?;
                let link = self
                    .register(connection, self.hub_id())
                    .await
                    .map_err(|error| (PeerAttempt::Admission, error))?;
                Ok::<_, (PeerAttempt, anyhow::Error)>((link, incoming))
            };
            let attempt = tokio::select! {
                _ = stop.changed() => break,
                attempt = attempt => attempt,
            };
            match attempt {
                Ok((link, incoming)) => {
                    if let Some(attempts) = failures.succeed() {
                        tracing::info!(
                            message = PEER_RECOVERED,
                            peer = %peer.identity.hub(),
                            attempts,
                        );
                    }
                    let served = self.clone().serve_link(link, incoming, stop.clone()).await;
                    if let Err(error) = served {
                        tracing::debug!(
                            peer = %peer.identity.hub(),
                            %error,
                            "Hub peer session ended unsuccessfully",
                        );
                    }
                }
                Err((stage, error)) => {
                    if self.is_closing() {
                        break;
                    }
                    if let Some(attempts) = failures.fail(stage) {
                        tracing::warn!(
                            message = PEER_ATTEMPT_FAILED,
                            peer = %peer.identity.hub(),
                            host = %address.host,
                            port = address.port.get(),
                            stage = stage.as_str(),
                            attempts,
                        );
                    }
                    tracing::debug!(
                        peer = %peer.identity.hub(),
                        stage = stage.as_str(),
                        %error,
                        "Hub peer connection attempt detail",
                    );
                }
            }
        }
        Ok(())
    }
    pub(crate) async fn accept_loop(
        self: Arc<Self>,
        listener: TcpListener,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut tasks = JoinSet::new();
        let capacity = Arc::new(Semaphore::new(MAX_PEERS));
        let pins = PeerPins::new(self.config.peers.iter().map(|peer| peer.identity).collect())?;
        let mut rejections = InboundRejections::default();
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                completed = tasks.join_next(), if !tasks.is_empty() => {
                    let Some(completed) = completed else { continue; };
                    match completed {
                        Ok((remote, false, Err(error))) => {
                            if let Some(suppressed) = rejections.reject(std::time::Instant::now()) {
                                tracing::warn!(
                                    message = INBOUND_PEER_REJECTED,
                                    ?remote,
                                    suppressed,
                                );
                            }
                            tracing::debug!(?remote, %error, "inbound peer rejection detail");
                        }
                        Ok((remote, true, Err(error))) => {
                            tracing::debug!(?remote, %error, "peer session ended unsuccessfully");
                        }
                        Ok((_, _, Ok(()))) => {}
                        Err(error) if !error.is_cancelled() => {
                            tracing::debug!(%error, "peer task failed");
                        }
                        Err(_) => {}
                    }
                },
                accepted = listener.accept() => {
                    if self.is_closing() { break; }
                    self.state.verify()?;
                    let (stream, _) = accepted?;
                    let Ok(permit) = capacity.clone().try_acquire_owned() else { continue; };
                    let node = self.clone(); let pins = pins.clone(); let stop = stop.clone();
                    tasks.spawn(async move {
                        let remote = stream.peer_addr().ok();
                        let setup = async {
                            match yolo_hub_peer_transport::accept_with_enrollment(
                                stream,
                                node.key.clone(),
                                pins,
                                node.enrollment.get().map(|service| service.verifier()),
                                Instant::now() + CONNECT_TIMEOUT,
                            )
                            .await?
                            {
                                yolo_hub_peer_transport::Accepted::Peer {
                                    connection,
                                    incoming,
                                } => {
                                    let _permit = permit;
                                    let initiator = connection.peer().hub();
                                    let link = node.register(connection, initiator).await?;
                                    node.serve_link(link, incoming, stop).await
                                }
                                yolo_hub_peer_transport::Accepted::Enrollment {
                                    connection,
                                    incoming,
                                } => {
                                    if let Some(service) = node.enrollment.get() {
                                        service.spawn(connection, incoming, permit);
                                    }
                                    Ok(())
                                }
                            }
                        }
                        .await;
                        match setup {
                            Err(error) => (remote, false, Err(error)),
                            Ok(()) => (remote, true, Ok(())),
                        }
                    });
                }
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/network_capacity.rs"]
mod integration_tests;
