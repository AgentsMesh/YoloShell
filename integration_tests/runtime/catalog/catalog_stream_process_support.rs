use serde_json::Value;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use yolo_runtime::host::{ListSessionsRequest, ListSessionsResult};
use yolo_runtime::protocol::ResponseEnvelope;

const FRAME_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
const MAXIMUM_RESPONSE_BYTES: usize = yolo_capnp_codec::MAX_MESSAGE_BYTES;

pub async fn respond_empty(stream: &mut yolo_runtime_rpc_testing::Peer) {
    let request = stream.request().await.unwrap();
    let payload: ListSessionsRequest = request.decode_payload().unwrap();
    assert!(!payload.include_archived);
    assert!(payload.require_complete);
    let response = ResponseEnvelope::success(
        &request,
        &ListSessionsResult {
            sessions: Vec::new(),
            complete: true,
        },
    )
    .unwrap();
    stream.respond(&response).await.unwrap();
}

pub fn write_catalog_frame(writer: &mut impl Write, value: &Value) {
    let payload = yolo_capnp_codec::encode(value).unwrap();
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .unwrap();
    writer.write_all(&payload).unwrap();
    writer.flush().unwrap();
}

pub fn read_catalog_frame(reader: &mut impl Read) -> Value {
    let mut header = [u8::default(); FRAME_LENGTH_BYTES];
    reader.read_exact(&mut header).unwrap();
    let length = u32::from_be_bytes(header) as usize;
    assert!(length <= MAXIMUM_RESPONSE_BYTES);
    let mut payload = vec![u8::default(); length];
    reader.read_exact(&mut payload).unwrap();
    yolo_capnp_codec::decode(&payload).unwrap()
}

pub struct ChildGuard {
    pub child: Child,
    waited: bool,
}

impl ChildGuard {
    pub fn spawn(command: &mut Command) -> Self {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Self {
            child: command.spawn().unwrap(),
            waited: false,
        }
    }

    pub fn wait(&mut self) -> std::process::ExitStatus {
        self.waited = true;
        self.child.wait().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.waited {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
