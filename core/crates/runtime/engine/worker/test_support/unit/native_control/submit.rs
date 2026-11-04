use super::*;
use crate::worker::native_control::NativeSubmitRequest;
use std::io::{self, Write};

pub(super) const PROMPT: &str = "hello\n世界";
pub(super) const PAYLOAD: &[u8] = b"\x1b[200~hello\n\xe4\xb8\x96\xe7\x95\x8c\x1b[201~\r";
pub(super) const PASTE_ONLY: &[u8] = b"\x1b[200~hello\n\xe4\xb8\x96\xe7\x95\x8c\x1b[201~";
pub(super) const SUBMIT_KEY: &[u8] = b"\r";
pub(super) const PASTE_ENABLED: &[u8] = b"\x1b[?2004h";
pub(super) const PASTE_DISABLED: &[u8] = b"\x1b[?2004l";
pub(super) const PARTIAL_BYTES: usize = 3;
pub(super) const ONE_WRITE: usize = 1;

pub(super) async fn submit(source: &NativeSource) -> NativeSubmitRequest {
    let interrupt = interrupt(source).await;
    NativeSubmitRequest {
        terminal_id: interrupt.terminal_id,
        runtime_epoch: interrupt.runtime_epoch,
        request_id: interrupt.request_id,
        generation: interrupt.generation,
        writer_capability: interrupt.writer_capability,
        witness: interrupt.witness,
        command_digest: yolo_crypto::Sha256Digest::capture(&PROMPT).unwrap(),
        prompt: yolo_agent_input::Prompt::new(PROMPT.into()).unwrap(),
    }
}

pub(super) fn submit_lookup(request: &NativeSubmitRequest) -> NativeControlLookupRequest {
    NativeControlLookupRequest {
        terminal_id: request.terminal_id,
        runtime_epoch: request.runtime_epoch,
        request_id: request.request_id,
        command_digest: request.command_digest,
    }
}

pub(super) fn enable_paste(source: &NativeSource) {
    lock_recover(&source.fixture.state.terminal).process(PASTE_ENABLED);
}

#[derive(Clone, Copy)]
pub(super) enum WriteBehavior {
    Complete,
    Partial,
    Interrupted,
    Blocked,
}

pub(super) type Writes = Arc<Mutex<Vec<Vec<u8>>>>;

struct Recorder {
    writes: Writes,
    behavior: WriteBehavior,
}

impl Write for Recorder {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        lock_recover(&self.writes).push(data.to_vec());
        match self.behavior {
            WriteBehavior::Complete => Ok(data.len()),
            WriteBehavior::Partial => Ok(PARTIAL_BYTES),
            WriteBehavior::Interrupted => Err(io::ErrorKind::Interrupted.into()),
            WriteBehavior::Blocked => Err(io::ErrorKind::WouldBlock.into()),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("native prompt must not flush or perform a second write")
    }
}

pub(super) fn record_writes(source: &NativeSource, behavior: WriteBehavior) -> Writes {
    let writes = Arc::new(Mutex::new(Vec::new()));
    *lock_recover(&source.fixture.state.writer) = Some(PtyInputWriter::test(Recorder {
        writes: Arc::clone(&writes),
        behavior,
    }));
    writes
}

pub(super) async fn assert_submit_unreserved(source: &NativeSource, request: &NativeSubmitRequest) {
    assert!(
        source
            .fixture
            .state
            .native_control_receipt(source.fixture.controller(), &submit_lookup(request),)
            .await
            .unwrap()
            .is_none()
    );
    child_is_live(source);
}
