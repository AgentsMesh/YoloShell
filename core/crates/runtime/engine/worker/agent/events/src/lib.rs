//! A Worker's agent-binding event stream for the Host's catalogue watch.
//!
//! The stream carries binding payloads only. Terminal output, screen resets
//! and writer leases stay on the attachment stream, so observing agent state
//! never reads the PTY plane.

use tokio::sync::broadcast;
use uuid::Uuid;
use yolo_capnp_rpc::{EventSource, Future, Value};
use yolo_runtime_protocol::AgentEvent;
use yolo_wire::EventEnvelope;

pub struct AgentEventSource {
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    events: broadcast::Receiver<AgentEvent>,
}

impl AgentEventSource {
    pub fn new(
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        events: broadcast::Receiver<AgentEvent>,
    ) -> Self {
        Self {
            terminal_id,
            runtime_epoch,
            events,
        }
    }
}

impl EventSource for AgentEventSource {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async move {
            let event = self.events.recv().await.map_err(|error| match error {
                broadcast::error::RecvError::Closed => yolo_capnp_rpc::Error::Closed,
                // A lagged observer cannot know which bindings it missed. End
                // the stream so the Host resubscribes and resynchronises.
                broadcast::error::RecvError::Lagged(_) => yolo_capnp_rpc::Error::Closed,
            })?;
            let envelope = EventEnvelope::new(
                &event.event_type,
                Some(self.terminal_id.to_string()),
                Some(self.runtime_epoch.to_string()),
                &event,
            )
            .map_err(|error| yolo_capnp_rpc::Error::Remote(error.to_string()))?;
            yolo_capnp_codec::to_value(&envelope).map_err(Into::into)
        })
    }
}
