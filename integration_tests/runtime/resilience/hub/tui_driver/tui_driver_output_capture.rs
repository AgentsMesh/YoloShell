use crate::hub::support::MAX_OBSERVED_BYTES;
use yolo_client::ClientError;
use yolo_runtime::protocol::{EventEnvelope, ProtocolError, event, from_value};

const COUNTER_STEP: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Failure {
    EndOfStream,
    Protocol,
    InvalidOutput,
    OutputGap,
    DriverExited,
    PendingBytesExceeded,
}

#[derive(Default)]
pub(super) struct Capture {
    pending: Vec<u8>,
    pub sequence: Option<u64>,
    pub chunks: usize,
    total_bytes: usize,
    pub failure: Option<Failure>,
}

impl Capture {
    pub fn record(&mut self, result: Result<EventEnvelope, ClientError>) -> bool {
        match result {
            Err(ClientError::Protocol(ProtocolError::EndOfStream)) => {
                self.failure = Some(Failure::EndOfStream);
            }
            Err(_) => self.failure = Some(Failure::Protocol),
            Ok(message) => match message.event.as_str() {
                event::TERMINAL_OUTPUT => {
                    match from_value::<yolo_runtime::model::OutputChunk>(message.payload) {
                        Ok(chunk) => {
                            self.sequence = Some(chunk.sequence);
                            self.chunks = self.chunks.saturating_add(COUNTER_STEP);
                            self.total_bytes = self.total_bytes.saturating_add(chunk.data.len());
                            if chunk.data.len()
                                > MAX_OBSERVED_BYTES.saturating_sub(self.pending.len())
                            {
                                self.failure = Some(Failure::PendingBytesExceeded);
                            } else {
                                self.pending.extend_from_slice(&chunk.data);
                            }
                        }
                        Err(_) => self.failure = Some(Failure::InvalidOutput),
                    }
                }
                event::OUTPUT_GAP => self.failure = Some(Failure::OutputGap),
                event::SESSION_EXITED => self.failure = Some(Failure::DriverExited),
                _ => {}
            },
        }
        self.failure.is_some()
    }

    pub fn take_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending)
    }

    pub fn has_pending_output(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn evidence(&self) -> String {
        format!(
            "last_sequence={:?} chunks={} pending_bytes={} total_bytes={} limit={} failure={:?}",
            self.sequence,
            self.chunks,
            self.pending.len(),
            self.total_bytes,
            MAX_OBSERVED_BYTES,
            self.failure,
        )
    }
}
