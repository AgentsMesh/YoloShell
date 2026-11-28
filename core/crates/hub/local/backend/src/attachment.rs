use crate::{
    LocalBackend, attachment_event::convert_event, errors::map_error, inventory::dimensions,
};
use yolo_hub_identity::{RuntimeTarget, TerminalId};
use yolo_hub_terminal_protocol::{
    AttachRequest, AttachResult, AttachResultFields, AttachSeed, MAX_ATTACH_OUTPUT_BYTES,
    OutputChunk, OutputChunkFields, OutputCursor, PublicError, TerminalEvent, TerminalHistory,
    TerminalHistoryFields, TerminalSize, TerminalSnapshot, TerminalSnapshotFields,
};
use yolo_runtime_protocol::FORCE_SNAPSHOT_AFTER_SEQUENCE;

pub struct Attachment {
    pub initial: AttachResult,
    target: RuntimeTarget,
    source: yolo_client::Attachment,
    cursor: OutputCursor,
    ended: bool,
}

impl LocalBackend {
    pub async fn attach(&self, request: AttachRequest) -> Result<Attachment, PublicError> {
        let target = request.target();
        if target.address.hub_id != self.hub {
            return Err(PublicError::OriginMismatch);
        }
        let after_sequence = match request.seed() {
            AttachSeed::History => 0,
            AttachSeed::Snapshot => request.cursor().map_or(
                FORCE_SNAPSHOT_AFTER_SEQUENCE,
                |cursor| cursor.after_sequence,
            ),
        };
        let mut source = self
            .client
            .attach(target.address.terminal_id.as_uuid(), after_sequence)
            .await
            .map_err(map_error)?;
        if source.runtime_epoch() != target.runtime_epoch.as_uuid() {
            return Err(PublicError::TargetMismatch);
        }
        if request.seed() == AttachSeed::History && source.initial.snapshot.is_none() {
            if let Some(size) = self.terminal_size(target.address.terminal_id).await {
                let bar = source
                    .initial
                    .next_sequence
                    .checked_sub(1)
                    .ok_or(PublicError::InvalidCursor)?;
                let data = concat_chunks(&source.initial.chunks)?;
                let history = TerminalHistory::new(TerminalHistoryFields {
                    target,
                    barrier_sequence: bar,
                    size,
                    data,
                })?;
                let initial = AttachResult::new(AttachResultFields {
                    target,
                    snapshot: None,
                    chunks: Vec::new(),
                    history: Some(history),
                    next_sequence: source.initial.next_sequence,
                    resynchronized: false,
                })?;
                let cursor = initial.validate_for_request(request)?;
                return Ok(Attachment {
                    initial,
                    target,
                    source,
                    cursor,
                    ended: false,
                });
            }
            source = self
                .client
                .attach(target.address.terminal_id.as_uuid(), FORCE_SNAPSHOT_AFTER_SEQUENCE)
                .await
                .map_err(map_error)?;
            if source.runtime_epoch() != target.runtime_epoch.as_uuid() {
                return Err(PublicError::TargetMismatch);
            }
        }
        let snapshot = source
            .initial
            .snapshot
            .as_ref()
            .map(|snapshot| {
                TerminalSnapshot::new(TerminalSnapshotFields {
                    target,
                    barrier_sequence: snapshot.barrier_sequence,
                    size: TerminalSize::new(snapshot.cols, snapshot.rows)?,
                    data: snapshot.data.clone(),
                })
            })
            .transpose()?;
        let chunks = source
            .initial
            .chunks
            .iter()
            .map(|chunk| chunk_from(target, chunk))
            .collect::<Result<Vec<_>, _>>()?;
        let initial = AttachResult::new(AttachResultFields {
            target,
            snapshot,
            chunks,
            history: None,
            next_sequence: source.initial.next_sequence,
            resynchronized: source.initial.resynchronized,
        })?;
        let cursor = initial.validate_for_request(request)?;
        Ok(Attachment {
            initial,
            target,
            source,
            cursor,
            ended: false,
        })
    }

    async fn terminal_size(&self, terminal_id: TerminalId) -> Option<TerminalSize> {
        let views = self.catalogue_views(false).await.ok()?;
        let view = views
            .iter()
            .find(|view| view.manifest.terminal_id == terminal_id.as_uuid())?;
        dimensions(self, view)
    }
}

pub(crate) fn concat_chunks(
    chunks: &[yolo_terminal_model::OutputChunk],
) -> Result<Vec<u8>, PublicError> {
    let mut data = Vec::new();
    for chunk in chunks {
        if data.len() + chunk.data.len() > MAX_ATTACH_OUTPUT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        data.extend_from_slice(&chunk.data);
    }
    Ok(data)
}


impl Attachment {
    pub async fn next_event(&mut self) -> Result<TerminalEvent, PublicError> {
        if self.ended {
            return Err(PublicError::Conflict);
        }
        loop {
            let translated = match self.source.next_event().await {
                Ok(event) => convert_event(self.target, event),
                Err(error) => {
                    super::attachment_event::connection_loss(self.target, error).map(Some)
                }
            };
            let event = match translated {
                Ok(event) => event,
                Err(error) => {
                    self.ended = true;
                    return Err(error);
                }
            };
            let Some(event) = event else {
                continue;
            };
            self.ended = event.is_terminal();
            match event.advance(self.cursor) {
                Ok(cursor) => self.cursor = cursor,
                Err(PublicError::OutputGap)
                    if matches!(event, TerminalEvent::ResyncRequired { .. }) => {}
                Err(PublicError::Unreachable)
                    if matches!(event, TerminalEvent::Unreachable { .. }) => {}
                Err(error) => {
                    self.ended = true;
                    return Err(error);
                }
            }
            return Ok(event);
        }
    }

    pub async fn detach(self) -> Result<(), PublicError> {
        self.source.detach().await.map_err(map_error)
    }
}

pub(crate) fn chunk_from(
    target: RuntimeTarget,
    chunk: &yolo_terminal_model::OutputChunk,
) -> Result<OutputChunk, PublicError> {
    if chunk.runtime_epoch != target.runtime_epoch.as_uuid() {
        return Err(PublicError::TargetMismatch);
    }
    OutputChunk::new(OutputChunkFields {
        target,
        sequence: chunk.sequence,
        data: chunk.data.clone(),
    })
}
