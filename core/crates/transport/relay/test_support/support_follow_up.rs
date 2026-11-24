use super::fragment_transport::Transport;
use crate::protocol::{EventEnvelope, ProtocolError, to_value};
use tokio::sync::{Notify, oneshot};
use yolo_runtime_rpc_testing::Peer;

pub(crate) struct FollowUpGate {
    reached: oneshot::Sender<()>,
    resume: std::sync::Arc<Notify>,
}

impl FollowUpGate {
    pub(crate) fn new(reached: oneshot::Sender<()>, resume: std::sync::Arc<Notify>) -> Self {
        Self { reached, resume }
    }

    pub(super) async fn wait(self) {
        let _ = self.reached.send(());
        self.resume.notified().await;
    }
}

pub(crate) struct FragmentedEvent {
    event: EventEnvelope,
    ready: oneshot::Receiver<()>,
    fragment_bytes: usize,
    fragment_sent: oneshot::Sender<()>,
    resume: std::sync::Arc<Notify>,
}

impl FragmentedEvent {
    pub(crate) fn new(
        event: EventEnvelope,
        ready: oneshot::Receiver<()>,
        fragment_bytes: usize,
        fragment_sent: oneshot::Sender<()>,
        resume: std::sync::Arc<Notify>,
    ) -> Self {
        Self {
            event,
            ready,
            fragment_bytes,
            fragment_sent,
            resume,
        }
    }

    async fn send(self, peer: &Peer, transport: &Transport) -> Result<(), ProtocolError> {
        let Self {
            event,
            ready,
            fragment_bytes,
            fragment_sent,
            resume,
        } = self;
        ready.await.map_err(|_| ProtocolError::EndOfStream)?;
        transport
            .pause_after(fragment_bytes, fragment_sent, resume)
            .map_err(ProtocolError::Io)?;
        peer.event(&event).await
    }
}

pub(crate) enum FollowUp {
    None,
    Attachment {
        events: Vec<EventEnvelope>,
        gate: Option<FollowUpGate>,
        disconnect: bool,
        malformed: bool,
    },
    Fragmented(FragmentedEvent),
}

impl FollowUp {
    pub(crate) async fn send(
        self,
        peer: &mut Peer,
        transport: &Transport,
    ) -> Result<bool, ProtocolError> {
        match self {
            Self::None => Ok(false),
            Self::Attachment {
                events,
                gate,
                disconnect,
                malformed,
            } => {
                if let Some(gate) = gate {
                    gate.wait().await;
                }
                for event in events {
                    peer.event(&event).await?;
                }
                if malformed {
                    let malformed = super::values::malformed_event();
                    peer.raw_event(to_value(&malformed)?).await?;
                }
                if disconnect || malformed {
                    peer.finish_events();
                }
                Ok(false)
            }
            Self::Fragmented(event) => {
                event.send(peer, transport).await?;
                Ok(false)
            }
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/support_follow_up_tests.rs"]
mod tests;
