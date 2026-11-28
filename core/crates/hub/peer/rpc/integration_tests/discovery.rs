use super::*;
use yolo_hub_peer_protocol::{FIRST_REVISION, PEER_PROTOCOL_VERSION};

struct LegacyDirectory {
    reads: AtomicUsize,
    waits: AtomicUsize,
    failure: RpcError,
}

impl Handler for LegacyDirectory {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let PeerRequest::Discover { request_id, .. } = request else {
                return Err(RpcError::Invalid);
            };
            Ok(Response {
                reply: PeerReply::Directory {
                    request_id,
                    revision: FIRST_REVISION,
                    origins: Vec::new(),
                },
                updates: None,
            })
        })
    }

    fn watch_directory(&self, _: PeerRequest) -> RpcFuture<'_, Response> {
        self.waits.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(self.failure.clone()) })
    }
}

#[tokio::test]
async fn only_an_unimplemented_watch_falls_back_once_on_the_same_link() {
    let request_id = Fixture::new().call().payload.request.fields().request_id;
    for failure in [RpcError::Unimplemented, RpcError::Invalid] {
        let old = failure == RpcError::Unimplemented;
        let peer = Arc::new(LegacyDirectory {
            reads: 0.into(),
            waits: 0.into(),
            failure,
        });
        let (client, server) = tokio::io::duplex(PIPE_BYTES);
        let server = tokio::spawn(yolo_hub_peer_rpc::serve(server, peer.clone()));
        let client = Client::connect(client, Instant::now() + TIMEOUT)
            .await
            .unwrap();
        for _ in 0..2 {
            let request = PeerRequest::Discover {
                request_id,
                version: PEER_PROTOCOL_VERSION,
                allowed_origins: Vec::new(),
            };
            let reply = client
                .observe_directory(request, Instant::now() + TIMEOUT, 0)
                .await;
            assert_eq!(reply.is_ok(), old);
        }
        assert_eq!(peer.reads.load(Ordering::SeqCst), if old { 2 } else { 0 });
        assert_eq!(peer.waits.load(Ordering::SeqCst), if old { 1 } else { 2 });
        drop(client);
        tokio::time::timeout(TIMEOUT, server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
