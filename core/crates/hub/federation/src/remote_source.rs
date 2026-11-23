use crate::{constants::*, node::now_ms};
use tokio::time::Instant;
use yolo_hub_dispatch::{EventFuture, TerminalSource};
use yolo_hub_peer_protocol::{Call, FIRST_STREAM_SEQUENCE, Signed};
use yolo_hub_peer_rpc::RemoteUpdates;
use yolo_hub_terminal_protocol::{
    AttachResult, FIRST_OUTPUT_SEQUENCE, OutputCursor, PublicError, TerminalEvent,
};

pub(crate) struct RemoteSource {
    pub initial: AttachResult,
    pub channel: RemoteUpdates,
    pub call: Signed<Call>,
    pub sequence: u64,
    pub cursor: OutputCursor,
}
impl RemoteSource {
    pub fn new(initial: AttachResult, channel: RemoteUpdates, call: Signed<Call>) -> Self {
        let cursor = OutputCursor {
            target: initial.fields().target,
            after_sequence: initial.fields().next_sequence - FIRST_OUTPUT_SEQUENCE,
        };
        Self {
            initial,
            channel,
            call,
            sequence: FIRST_STREAM_SEQUENCE,
            cursor,
        }
    }
}
impl TerminalSource for RemoteSource {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(async move {
            let update = self
                .channel
                .next(Instant::now() + OPERATION_TIMEOUT)
                .await
                .map_err(|_| PublicError::Unreachable)?;
            update
                .validate_for(
                    &self.call,
                    self.sequence,
                    now_ms().map_err(|_| PublicError::Internal)?,
                )
                .map_err(|_| PublicError::InvalidPayload)?;
            self.sequence = self
                .sequence
                .checked_add(FIRST_STREAM_SEQUENCE)
                .ok_or(PublicError::InvalidCursor)?;
            match update.payload.event.advance(self.cursor) {
                Ok(cursor) => self.cursor = cursor,
                Err(PublicError::OutputGap)
                    if matches!(update.payload.event, TerminalEvent::ResyncRequired { .. }) => {}
                Err(PublicError::Unreachable)
                    if matches!(update.payload.event, TerminalEvent::Unreachable { .. }) => {}
                Err(error) => return Err(error),
            }
            Ok(update.payload.event)
        })
    }
}
