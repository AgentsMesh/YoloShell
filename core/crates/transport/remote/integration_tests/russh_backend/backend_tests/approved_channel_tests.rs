use super::*;
use crate::model::HostStatus;
use crate::protocol::{RequestEnvelope, method};
use crate::remote::constants::SHELL_SEPARATOR;
use russh::ChannelMsg;
use tokio::io::AsyncReadExt;

const EOF_PROBE_BYTES: usize = 1;
const NO_BYTES_READ: usize = 0;

#[test]
fn memory_runtime_accepts_only_the_explicit_host_bridge_invocation() {
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    let command = super::super::command_line::runtime_command(&runtime).unwrap();
    assert!(super::server_command::runtime(command.as_bytes()));
    let role = yolo_launch_contract::ProcessRole::Host
        .arguments()
        .map(|argument| format!("'{argument}'"))
        .join(SHELL_SEPARATOR);
    let remote = yolo_launch_contract::ProcessRole::Remote
        .arguments()
        .map(|argument| format!("'{argument}'"))
        .join(SHELL_SEPARATOR);
    for invalid in [
        command.replace(&format!("{role}{SHELL_SEPARATOR}"), ""),
        command.replace(&role, &remote),
        format!("{command}{SHELL_SEPARATOR}{role}"),
    ] {
        assert!(!super::server_command::runtime(invalid.as_bytes()));
    }
}

#[tokio::test]
async fn buffered_runtime_stream_replays_early_bytes_before_live_data() {
    let fixture = connect_memory(false).await.unwrap();
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    let mut channel = fixture
        .connection
        .session
        .channel_open_session()
        .await
        .unwrap();
    let command = super::super::command_line::runtime_command(&runtime).unwrap();
    let mut approved = super::super::channel_request::exec(&mut channel, command)
        .await
        .unwrap();
    approved
        .push(ChannelMsg::Data {
            data: APPROVAL_COMMAND_PREFIX.to_vec().into(),
        })
        .unwrap();
    let mut stream = approved.into_stream(channel);
    let mut prefix = vec![0; APPROVAL_COMMAND_PREFIX.len()];
    stream.read_exact(&mut prefix).await.unwrap();
    assert_eq!(prefix, APPROVAL_COMMAND_PREFIX);
    let request = RequestEnvelope::new(
        BUFFERED_RUNTIME_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let deadline = crate::remote::RemoteDeadline::operation().instant();
    let mut rpc = yolo_runtime_rpc::RuntimeConnection::open(stream, deadline).await.unwrap();
    let response = rpc.exchange(&request, deadline).await.unwrap();
    assert!(response.ok);
    let status: HostStatus = response.decode_payload().unwrap();
    assert_eq!(status.host_pid, MEMORY_HOST_PID);
    rpc.close();
}

#[tokio::test]
async fn buffered_eof_ends_after_replaying_accepted_data() {
    let fixture = connect_memory(false).await.unwrap();
    let channel = fixture
        .connection
        .session
        .channel_open_session()
        .await
        .unwrap();
    let mut approved = super::super::approved_channel::ApprovalBuffer::new();
    approved
        .push(ChannelMsg::Data {
            data: APPROVAL_COMMAND_PREFIX.to_vec().into(),
        })
        .unwrap();
    approved.push(ChannelMsg::Eof).unwrap();
    let mut stream = approved.into_stream(channel);
    let mut empty = [];
    assert_eq!(stream.read(&mut empty).await.unwrap(), NO_BYTES_READ);
    let mut prefix = vec![0; APPROVAL_COMMAND_PREFIX.len()];
    stream.read_exact(&mut prefix).await.unwrap();
    let mut probe = [0_u8; EOF_PROBE_BYTES];

    assert_eq!(prefix, APPROVAL_COMMAND_PREFIX);
    assert_eq!(stream.read(&mut probe).await.unwrap(), NO_BYTES_READ);
}
