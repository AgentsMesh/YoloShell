use super::fixture;
use yolo_remote::{ChannelIdentity, OpenedRemoteChannel};
use yolo_runtime::host::HelloResult;
use yolo_runtime::model::HostStatus;
use yolo_runtime::protocol::{RequestEnvelope, ResponseEnvelope, method};

const PROBE_CHANNEL_CAPACITY: usize = 65_536;

pub(super) fn opened(identity: ChannelIdentity) -> OpenedRemoteChannel {
    let (client, server) = tokio::io::duplex(PROBE_CHANNEL_CAPACITY);
    tokio::spawn(serve(server));
    OpenedRemoteChannel {
        identity,
        stream: Box::new(client),
    }
}

async fn serve(stream: tokio::io::DuplexStream) -> anyhow::Result<()> {
    let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
    let hello: RequestEnvelope = stream.request().await?;
    anyhow::ensure!(hello.method == method::HELLO);
    stream
        .respond(&ResponseEnvelope::success(
            &hello,
            &HelloResult {
                server_name: yolo_runtime::protocol::HOST_SERVER_NAME.into(),
                protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
                capabilities: capabilities(),
            },
        )?)
        .await?;
    let status: RequestEnvelope = stream.request().await?;
    anyhow::ensure!(status.method == method::HOST_STATUS);
    stream
        .respond(&ResponseEnvelope::success(
            &status,
            &HostStatus {
                protocol_version: yolo_runtime::model::PROTOCOL_VERSION,
                boot_id: String::new(),
                host_pid: u32::default(),
                controller_generation: u64::default(),
                home: fixture::TEST_HOME.into(),
                capabilities: capabilities(),
            },
        )?)
        .await?;
    Ok(())
}

fn capabilities() -> Vec<String> {
    [
        yolo_runtime::protocol::capability::DURABLE_PTY,
        yolo_runtime::protocol::capability::HOST_ADOPTION,
        yolo_runtime::protocol::capability::CAPNP_BINARY,
        yolo_runtime::protocol::capability::OUTPUT_JOURNAL,
        yolo_runtime::protocol::capability::SSH_STDIO_BRIDGE,
        yolo_runtime::protocol::capability::WRITER_LEASE_V2,
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
