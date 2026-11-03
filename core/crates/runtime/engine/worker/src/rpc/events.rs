use super::super::*;
use yolo_capnp_rpc::{EventSource, Future, Value};

pub(super) struct Events {
    state: Arc<WorkerState>,
    receiver: broadcast::Receiver<WorkerEvent>,
    barrier: u64,
    _permit: tokio::sync::OwnedSemaphorePermit,
    completed: Option<ExitRecord>,
    ended: bool,
}

impl Events {
    pub(super) fn new(
        state: Arc<WorkerState>,
        receiver: broadcast::Receiver<WorkerEvent>,
        barrier: u64,
        permit: tokio::sync::OwnedSemaphorePermit,
        completed: Option<ExitRecord>,
    ) -> Self {
        Self {
            state,
            receiver,
            barrier,
            _permit: permit,
            completed,
            ended: false,
        }
    }
    fn envelope(
        &self,
        event: &str,
        payload: &impl Serialize,
    ) -> std::result::Result<EventEnvelope, yolo_capnp_rpc::Error> {
        let (terminal, epoch) = (self.state.terminal_id(), self.state.runtime_epoch());
        EventEnvelope::new(event, Some(terminal.to_string()), Some(epoch.to_string()), payload)
            .map_err(|error| yolo_capnp_rpc::Error::Remote(error.to_string()))
    }
}

impl EventSource for Events {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async move {
            if self.ended {
                return Err(yolo_capnp_rpc::Error::Closed);
            }
            let next = if let Some(exit) = self.completed.take() {
                Ok(WorkerEvent::Exited(exit))
            } else {
                recv_after_barrier(&mut self.receiver, self.barrier).await
            };
            let envelope = match next {
                Ok(WorkerEvent::Output(chunk)) => self.envelope(event::TERMINAL_OUTPUT, &chunk)?,
                Ok(WorkerEvent::ScreenReset(gap)) => {
                    self.ended = true;
                    self.envelope(event::OUTPUT_GAP, &gap)?
                }
                Ok(WorkerEvent::Exited(exit)) => {
                    self.ended = true;
                    self.envelope(event::SESSION_EXITED, &exit)?
                }
                Ok(WorkerEvent::WriterLease { event_type, payload }) => self.envelope(event_type, &payload)?,
                Ok(WorkerEvent::Agent { event_type, binding }) => self.envelope(event_type, &binding)?,
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    self.ended = true;
                    self.envelope(
                        event::OUTPUT_GAP,
                        &OutputGapPayload {
                            skipped,
                            resync_required: true,
                        },
                    )?
                }
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(yolo_capnp_rpc::Error::Closed);
                }
            };
            yolo_capnp_codec::to_value(&envelope).map_err(Into::into)
        })
    }
}
