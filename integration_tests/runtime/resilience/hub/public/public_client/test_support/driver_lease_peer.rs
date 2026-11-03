use super::{constants::DRIVER_NAME, deadline};
use std::time::Duration;
use tokio::{net::UnixListener, time::timeout_at};
use uuid::Uuid;
use yolo_runtime::{
    lease::constants::{MAX_TTL_MS, RENEW_AFTER_DIVISOR},
    model::{WriterCapability, WriterLease, WriterLeaseGrant},
    protocol::{ProtocolError, RequestEnvelope, ResponseEnvelope, method},
    worker::{LeaseRequest, RuntimeRef},
};

const SOCKET_ROOT: &str = "/tmp";
const SOCKET_NAME: &str = "driver-renewal.sock";
pub const GENERATION_STEP: u64 = 1;
pub const RENEWAL_ROUNDS: u64 = RENEW_AFTER_DIVISOR + GENERATION_STEP;
pub const EXPIRED_DETAIL: &str = "original driver grant expired";

pub struct Peer {
    _directory: tempfile::TempDir,
    listener: UnixListener,
    pub client: yolo_client::Client,
    pub target: RuntimeRef,
    pub grant: WriterLeaseGrant,
}

impl Peer {
    pub fn new() -> Self {
        let directory = tempfile::tempdir_in(SOCKET_ROOT).unwrap();
        let socket = directory.path().join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        let grant = WriterLeaseGrant {
            lease: WriterLease {
                holder_id: DRIVER_NAME.into(),
                controller_instance_id: Some(Uuid::new_v4().to_string()),
                generation: GENERATION_STEP,
                expires_at_ms: yolo_runtime::model::now_ms() + i64::try_from(MAX_TTL_MS).unwrap(),
                ttl_ms: MAX_TTL_MS,
                renew_after_ms: MAX_TTL_MS / RENEW_AFTER_DIVISOR,
            },
            writer_capability: WriterCapability::generate(),
        };
        Self {
            _directory: directory,
            listener,
            client: yolo_client::Client::new(socket),
            target: RuntimeRef {
                terminal_id: Uuid::new_v4(),
                runtime_epoch: Uuid::new_v4(),
            },
            grant,
        }
    }

    pub async fn renewal(&self) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
        // Drive the real renewal timer without spending the lease TTL in wall time.
        tokio::time::pause();
        tokio::time::advance(Duration::from_millis(self.grant.renew_after_ms)).await;
        tokio::time::resume();
        self.request(method::RENEW_WRITER).await
    }

    pub async fn request(
        &self,
        expected: &str,
    ) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
        let (stream, _) = timeout_at(deadline(), self.listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        let request = timeout_at(deadline(), stream.request())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(request.method, expected);
        let lease: LeaseRequest = request.decode_payload().unwrap();
        assert_eq!(lease.terminal_id, self.target.terminal_id);
        assert_eq!(lease.runtime_epoch, self.target.runtime_epoch);
        assert_eq!(lease.holder_id, self.grant.holder_id);
        assert_eq!(
            lease.controller_instance_id,
            self.grant.controller_instance_id
        );
        assert_eq!(lease.generation, Some(self.grant.generation));
        assert_eq!(lease.writer_capability, self.grant.writer_capability);
        if expected == method::RENEW_WRITER {
            assert_eq!(lease.ttl_ms, Some(self.grant.ttl_ms));
        } else {
            assert_eq!(lease.ttl_ms, None);
        }
        (stream, request)
    }
}

pub async fn respond(mut stream: yolo_runtime_rpc_testing::Peer, response: ResponseEnvelope) {
    timeout_at(deadline(), stream.respond(&response))
        .await
        .unwrap()
        .unwrap();
    // The client drops this one-RPC connection after consuming the reply. That
    // is a causal barrier before advancing the next scheduled renewal.
    assert!(matches!(
        timeout_at(deadline(), stream.request()).await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}
