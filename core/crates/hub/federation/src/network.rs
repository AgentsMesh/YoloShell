use crate::{Federation, constants::*};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    net::UnixListener,
    sync::{Semaphore, watch},
    task::JoinSet,
    time::{Instant, interval},
};
use yolo_hub_federation_settings::{MAX_PEERS, Peer};
use yolo_hub_peer_diagnostics::{
    INBOUND_PEER_REJECTED, InboundRejections, OutboundFailures, PEER_ATTEMPT_FAILED,
    PEER_CONNECTED, PEER_RECOVERED, PeerAttempt,
};
use yolo_hub_peer_link::{Admission, accept, connect};

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
                let client = connect(
                    transport,
                    self.state.peer_identity(),
                    self.state.as_ref(),
                    peer.identity,
                    deadline,
                )
                .await
                .map_err(|error| (PeerAttempt::Handshake, error.into()))?;
                let link = self
                    .register(peer.identity, client)
                    .await
                    .map_err(|error| (PeerAttempt::Admission, error))?;
                Ok::<_, (PeerAttempt, anyhow::Error)>(link)
            };
            let attempt = tokio::select! {
                _ = stop.changed() => break,
                attempt = attempt => attempt,
            };
            match attempt {
                Ok(link) => {
                    match failures.succeed() {
                        Some(attempts) => tracing::info!(
                            message = PEER_RECOVERED,
                            peer = %peer.identity.hub(),
                            attempts,
                        ),
                        None => tracing::info!(
                            message = PEER_CONNECTED,
                            peer = %peer.identity.hub(),
                            ssh = address.ssh_target().unwrap_or("-"),
                        ),
                    }
                    let served = self.clone().serve_link(link, stop.clone()).await;
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
                            ssh = address.ssh_target().unwrap_or("-"),
                            profile = address.profile_id().unwrap_or("-"),
                            socket = %address.socket().display(),
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
        listener: UnixListener,
        mut stop: watch::Receiver<bool>,
    ) -> Result<()> {
        let mut tasks = JoinSet::new();
        let capacity = Arc::new(Semaphore::new(MAX_PEERS));
        let allowed: Arc<Vec<_>> = Arc::new(
            self.config
                .peers
                .iter()
                .map(|peer| peer.identity)
                .collect(),
        );
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
                    let node = self.clone(); let allowed = allowed.clone();
                    tasks.spawn(async move {
                        let remote = stream.peer_addr().ok();
                        let setup = async {
                            let admission = if node.config.ssh_peers.require_registration {
                                Admission::Only(&allowed)
                            } else {
                                Admission::Any
                            };
                            let (peer, stream) = accept(
                                stream,
                                node.state.peer_identity(),
                                node.state.as_ref(),
                                admission,
                                Instant::now() + CONNECT_TIMEOUT,
                            )
                            .await?;
                            let _permit = permit;
                            node.serve_peer(peer, stream).await
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
