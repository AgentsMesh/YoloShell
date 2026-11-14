use super::super::constants::*;
use yolo_client::ClientError;
use yolo_runtime::protocol::{EventEnvelope, ProtocolError, event, from_value};

#[derive(Clone, Copy, Debug)]
enum EventKind {
    Output,
    Gap,
    Exited,
    Other,
}

#[derive(Default)]
pub(super) struct Capture {
    bytes: Vec<u8>,
    pub sequence: Option<u64>,
    pub chunks: usize,
    last_event: Option<EventKind>,
    failure: Option<&'static str>,
}

impl Capture {
    pub fn record(&mut self, result: Result<EventEnvelope, ClientError>) -> bool {
        match result {
            Err(ClientError::Protocol(ProtocolError::EndOfStream)) => {
                self.failure = Some(CAPTURE_EOF)
            }
            Err(_) => self.failure = Some(CAPTURE_STREAM_FAILURE),
            Ok(message) => match message.event.as_str() {
                event::TERMINAL_OUTPUT => {
                    self.last_event = Some(EventKind::Output);
                    match from_value::<yolo_runtime::model::OutputChunk>(message.payload) {
                        Ok(chunk) => {
                            self.sequence = Some(chunk.sequence);
                            self.chunks += 1;
                            if chunk.data.len() > MAX_CAPTURE_BYTES.saturating_sub(self.bytes.len())
                            {
                                self.failure = Some(CAPTURE_OVERFLOW);
                            } else {
                                self.bytes.extend_from_slice(&chunk.data);
                            }
                        }
                        Err(_) => self.failure = Some(CAPTURE_INVALID_OUTPUT),
                    }
                }
                event::OUTPUT_GAP => {
                    self.last_event = Some(EventKind::Gap);
                    self.failure = Some(CAPTURE_GAP);
                }
                event::SESSION_EXITED => {
                    self.last_event = Some(EventKind::Exited);
                    self.failure = Some(CAPTURE_EXITED);
                }
                _ => self.last_event = Some(EventKind::Other),
            },
        }
        self.failure.is_some()
    }

    pub fn healthy(&self) -> Result<(), &'static str> {
        self.failure.map_or(Ok(()), Err)
    }

    pub fn evidence(&self) -> String {
        format!(
            "last_sequence={:?} last_event={:?} chunks={} buffered_bytes={} limit={}",
            self.sequence,
            self.last_event,
            self.chunks,
            self.bytes.len(),
            MAX_CAPTURE_BYTES
        )
    }

    pub fn complete_commands(&self, expected: &[&[u8]]) -> bool {
        let positions = self.command_positions();
        positions.len() >= expected.len()
            && positions.iter().zip(expected).all(|(position, command)| {
                self.bytes
                    .get(*position..*position + command.len())
                    .is_some()
            })
    }

    pub fn assert_commands(&self, expected: &[&[u8]]) -> Result<(), &'static str> {
        self.healthy()?;
        let positions = self.command_positions();
        if positions.len() != expected.len() {
            return Err(CAPTURE_COMMAND_COUNT);
        }
        for (position, command) in positions.into_iter().zip(expected) {
            if self.bytes.get(position..position + command.len()) != Some(*command) {
                return Err(CAPTURE_COMMAND_MISMATCH);
            }
        }
        Ok(())
    }

    fn command_positions(&self) -> Vec<usize> {
        self.bytes
            .windows(CLIPBOARD_PREFIX.len())
            .enumerate()
            .filter_map(|(position, bytes)| (bytes == CLIPBOARD_PREFIX).then_some(position))
            .collect()
    }
}
