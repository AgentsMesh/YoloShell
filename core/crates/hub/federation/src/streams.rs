use crate::{Federation, constants::*, lifetime::OwnedGeneration, node::now_ms};
use anyhow::Result;
use std::sync::Arc;
use tokio::{
    sync::mpsc,
    time::{Instant, interval},
};
use yolo_hub_dispatch::TerminalSource;
use yolo_hub_peer_protocol::{Call, FIRST_STREAM_SEQUENCE, Purpose, Signed, TerminalUpdate};
use yolo_hub_peer_rpc::{RemoteUpdates, RpcError, RpcFuture, Updates};
use yolo_hub_terminal_protocol::{FIRST_OUTPUT_SEQUENCE, OutputCursor, TerminalEvent};

impl Federation {
    pub(crate) async fn write_origin_events(
        &self,
        call: &Signed<Call>,
        source: &mut dyn TerminalSource,
        sender: &mpsc::Sender<Result<Signed<TerminalUpdate>, RpcError>>,
    ) -> Result<()> {
        let mut tick = interval(HEARTBEAT_INTERVAL);
        let initial = source.initial().fields();
        let mut cursor = OutputCursor {
            target: initial.target,
            after_sequence: initial.next_sequence - FIRST_OUTPUT_SEQUENCE,
        };
        let mut sequence = FIRST_STREAM_SEQUENCE;
        loop {
            let next = source.next_event();
            tokio::pin!(next);
            loop {
                let (event, source_completed) = tokio::select! {
                    result = &mut next => { let event = result?; cursor = advance_cursor(&event, cursor)?; (event, true) },
                    _ = tick.tick() => (TerminalEvent::Heartbeat { cursor }, false),
                };
                self.require_open()?;
                let now = now_ms()?;
                let required =
                    yolo_hub_peer_protocol::required_rights(&call.payload.request.fields().command)
                        .ok_or_else(|| anyhow::anyhow!(LOCAL_MANAGEMENT_STREAM_DETAIL))?;
                call.payload.authority.authorize(
                    self.state.peer_identity(),
                    self.policy_epoch,
                    call.signer,
                    now,
                    &required,
                )?;
                let terminal = event.is_terminal();
                let update = Signed::new(
                    TerminalUpdate {
                        call_fingerprint: call.fingerprint()?,
                        sequence,
                        issued_at_ms: now,
                        event,
                    },
                    Purpose::Terminal,
                    self.state.as_ref(),
                )?;
                sender
                    .send(Ok(update))
                    .await
                    .map_err(|_| anyhow::anyhow!(INVALID_DISCOVERY))?;
                sequence = sequence
                    .checked_add(FIRST_STREAM_SEQUENCE)
                    .ok_or_else(|| anyhow::anyhow!(EXHAUSTED))?;
                if terminal {
                    return Ok(());
                }
                if source_completed {
                    break;
                }
            }
        }
    }
    pub(crate) fn bridge(
        self: Arc<Self>,
        call: Signed<Call>,
        upstream: RemoteUpdates,
    ) -> Result<Box<dyn Updates>> {
        Ok(Box::new(Bridge {
            call,
            upstream,
            sequence: FIRST_STREAM_SEQUENCE,
            ended: false,
            generation: OwnedGeneration::new(self)?,
        }))
    }
}
struct Bridge {
    call: Signed<Call>,
    upstream: RemoteUpdates,
    sequence: u64,
    ended: bool,
    generation: OwnedGeneration,
}
impl Updates for Bridge {
    fn next(&mut self) -> RpcFuture<'_, Signed<TerminalUpdate>> {
        Box::pin(async {
            if self.ended {
                return Err(RpcError::Closed);
            }
            let update = self
                .upstream
                .next(Instant::now() + OPERATION_TIMEOUT)
                .await?;
            update
                .validate_for(
                    &self.call,
                    self.sequence,
                    now_ms().map_err(|_| RpcError::Invalid)?,
                )
                .map_err(|_| RpcError::Invalid)?;
            self.generation
                .node()
                .require_open()
                .map_err(|_| RpcError::Closed)?;
            self.sequence = self
                .sequence
                .checked_add(FIRST_STREAM_SEQUENCE)
                .ok_or(RpcError::Invalid)?;
            self.ended = update.payload.event.is_terminal();
            Ok(update)
        })
    }
}
fn advance_cursor(event: &TerminalEvent, cursor: OutputCursor) -> Result<OutputCursor> {
    match event.advance(cursor) {
        Ok(cursor) => Ok(cursor),
        Err(yolo_hub_terminal_protocol::PublicError::OutputGap)
            if matches!(event, TerminalEvent::ResyncRequired { .. }) =>
        {
            Ok(cursor)
        }
        Err(yolo_hub_terminal_protocol::PublicError::Unreachable)
            if matches!(event, TerminalEvent::Unreachable { .. }) =>
        {
            Ok(cursor)
        }
        Err(error) => Err(error.into()),
    }
}
