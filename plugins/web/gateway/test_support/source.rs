use super::TERMINAL_BYTES;
use yolo_hub_dispatch::{EventFuture, TerminalSource};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::{
    AttachResult, AttachResultFields, OutputChunk, OutputChunkFields, TerminalEvent,
};

pub struct Source {
    initial: AttachResult,
    sequence: u64,
}

impl Source {
    pub fn new(target: RuntimeTarget) -> Self {
        let chunk = OutputChunk::new(OutputChunkFields {
            target,
            sequence: 1,
            data: TERMINAL_BYTES.to_vec(),
        })
        .unwrap();
        Self {
            initial: AttachResult::new(AttachResultFields {
                target,
                snapshot: None,
                chunks: vec![chunk],
                history: None,
                next_sequence: 2,
                resynchronized: false,
            })
            .unwrap(),
            sequence: 2,
        }
    }
}

impl TerminalSource for Source {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        let target = self.initial.fields().target;
        let sequence = self.sequence;
        self.sequence += 1;
        Box::pin(async move {
            Ok(TerminalEvent::Output {
                chunk: OutputChunk::new(OutputChunkFields {
                    target,
                    sequence,
                    data: vec![b'x'; yolo_web_gateway::STREAM_CHUNK_BYTES],
                })?,
            })
        })
    }
}
