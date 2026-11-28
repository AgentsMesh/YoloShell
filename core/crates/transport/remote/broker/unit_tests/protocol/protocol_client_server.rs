use super::{CONFIG_REVISION, HOST_ID};
use crate::remote::CommandOutput;
use crate::remote_broker::constants::PRIVATE_FILE_MODE;
use crate::remote_broker::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::net::UnixListener;
use uuid::Uuid;

const MANAGEMENT_SOCKET: &str = "management.sock";
const TEST_INSTANCE_ID: u128 = 1;
const WRONG_INSTANCE_ID: u128 = 2;
pub(super) const CONNECTION_GENERATION: u64 = 3;
const FAILURE_CODE: &str = "denied";
const FAILURE_MESSAGE: &str = "blocked";
const WRONG_SHUTDOWN_REVISION: &str = "different-revision";
const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

#[derive(Clone, Copy)]
pub(super) enum ReplyKind {
    Status,
    Bootstrap,
    PendingStatus,
    WrongResult,
    WrongVersion,
    WrongId,
    Failure,
    WrongShutdownRevision,
    WrongShutdownInstance,
    LegacyStatus,
}

pub(super) struct Server {
    _root: tempfile::TempDir,
    pub(super) path: PathBuf,
    pub(super) task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub(super) async fn start(kind: ReplyKind) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(MANAGEMENT_SOCKET);
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
        let task = tokio::spawn(serve(listener, kind));
        Self {
            _root: root,
            path,
            task,
        }
    }
}

async fn serve(listener: UnixListener, kind: ReplyKind) {
    let (mut stream, _) = listener.accept().await.unwrap();
    let request: BrokerRequestEnvelope = crate::protocol::read_frame(&mut stream).await.unwrap();
    let instance_id = Uuid::from_u128(TEST_INSTANCE_ID);
    let mut status = BrokerStatus::disconnected(
        instance_id,
        HOST_ID.into(),
        CONFIG_REVISION.into(),
        CONNECTION_GENERATION,
    );
    if matches!(kind, ReplyKind::LegacyStatus) {
        status.capabilities.clear();
    }
    if matches!(kind, ReplyKind::PendingStatus) {
        status.adopted = false;
    }
    let mut response = match kind {
        ReplyKind::Failure => {
            BrokerResponse::failure(request.request_id, FAILURE_CODE, FAILURE_MESSAGE)
        }
        ReplyKind::WrongResult => BrokerResponse::success(
            request.request_id,
            BrokerResult::Command {
                connection_id: Uuid::nil(),
                output: CommandOutput {
                    stdout: vec![],
                    stderr: vec![],
                    exit_status: None,
                    timed_out: false,
                    truncated: false,
                }
                .into(),
            },
        ),
        ReplyKind::Bootstrap => {
            BrokerResponse::success(request.request_id, BrokerResult::Bootstrap(status))
        }
        _ => BrokerResponse::success(request.request_id, BrokerResult::Status(status)),
    };
    if matches!(kind, ReplyKind::WrongVersion) {
        response.protocol_version += INCOMPATIBLE_PROTOCOL_VERSION_OFFSET;
    }
    if matches!(kind, ReplyKind::WrongId) {
        response.request_id = Uuid::nil();
    }
    crate::protocol::write_frame(&mut stream, &response)
        .await
        .unwrap();
    if matches!(kind, ReplyKind::WrongShutdownRevision) {
        reply_shutdown(&mut stream, instance_id, WRONG_SHUTDOWN_REVISION).await;
    }
    if matches!(kind, ReplyKind::WrongShutdownInstance) {
        reply_shutdown(
            &mut stream,
            Uuid::from_u128(WRONG_INSTANCE_ID),
            CONFIG_REVISION,
        )
        .await;
    }
    if matches!(kind, ReplyKind::LegacyStatus) {
        reply_shutdown(&mut stream, instance_id, CONFIG_REVISION).await;
    }
}

async fn reply_shutdown(stream: &mut tokio::net::UnixStream, instance_id: Uuid, revision: &str) {
    let request: BrokerRequestEnvelope = crate::protocol::read_frame(stream).await.unwrap();
    let response = BrokerResponse::success(
        request.request_id,
        BrokerResult::Shutdown {
            config_revision: revision.into(),
            instance_id,
        },
    );
    crate::protocol::write_frame(stream, &response)
        .await
        .unwrap();
}
