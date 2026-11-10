use crate::client::Client;
use crate::model::WriterLease;
use crate::protocol::{RequestEnvelope, to_value};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::net::UnixListener;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use yolo_runtime_rpc_testing::Peer;

#[path = "support_fragment_transport.rs"]
mod fragment_transport;

#[path = "support_values.rs"]
mod values;
use values::{requested_grant, requested_grant_with_ttl, success};

#[path = "support_follow_up.rs"]
mod follow_up;
pub(crate) use follow_up::FollowUpGate;
use follow_up::{FollowUp, FragmentedEvent};

#[path = "support_reply.rs"]
mod reply;
pub(crate) use reply::{RenewalReply, Reply};

pub(crate) struct FakeHost {
    _directory: TempDir,
    socket: PathBuf,
    requests: Arc<Mutex<Vec<RequestEnvelope>>>,
    request_arrived: watch::Sender<usize>,
    task: JoinHandle<()>,
}

impl FakeHost {
    pub(crate) async fn start(replies: Vec<Reply>) -> Self {
        Self::configured(replies, None).await
    }

    pub(crate) async fn start_renewing(replies: Vec<Reply>, renewal: RenewalReply) -> Self {
        Self::configured(replies, Some(renewal)).await
    }

    async fn configured(replies: Vec<Reply>, renewal: Option<RenewalReply>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("host.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let shared_requests = Arc::clone(&requests);
        let initial_request_count = requests.lock().unwrap().len();
        let (request_arrived, _) = watch::channel(initial_request_count);
        let shared_request_arrived = request_arrived.clone();
        let replies = Arc::new(Mutex::new(VecDeque::from(replies)));
        let stored_lease = Arc::new(Mutex::new(None::<WriterLease>));
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let replies = Arc::clone(&replies);
                let requests = Arc::clone(&shared_requests);
                let request_arrived = shared_request_arrived.clone();
                let stored_lease = Arc::clone(&stored_lease);
                tokio::spawn(async move {
                    let (stream, transport) = fragment_transport::Transport::new(stream);
                    let mut peer = Peer::new(stream);
                    while let Ok(request) = peer.request().await {
                        let request_count = {
                            let mut requests = requests.lock().unwrap();
                            requests.push(request.clone());
                            requests.len()
                        };
                        request_arrived.send_replace(request_count);
                        let reply = renewal
                            .and_then(|plan| {
                                plan.for_request(&request, stored_lease.lock().unwrap().as_ref())
                            })
                            .unwrap_or_else(|| {
                                replies.lock().unwrap().pop_front().expect("scripted reply")
                            });
                        let (response, follow_up) = match reply {
                            Reply::Disconnect => return,
                            Reply::Pending => std::future::pending().await,
                            Reply::Success(payload) => (success(&request, payload), FollowUp::None),
                            Reply::GatedSuccess(payload, gate) => {
                                gate.wait().await;
                                (success(&request, payload), FollowUp::None)
                            }
                            Reply::Error(error) => (
                                crate::protocol::ResponseEnvelope::failure(&request, error),
                                FollowUp::None,
                            ),
                            Reply::InputDelivered => {
                                (reply::input_delivered(&request), FollowUp::None)
                            }
                            Reply::Lease(generation) => {
                                let grant =
                                    requested_grant(request.decode_payload().unwrap(), generation);
                                *stored_lease.lock().unwrap() = Some(grant.lease.clone());
                                (success(&request, to_value(&grant).unwrap()), FollowUp::None)
                            }
                            Reply::LeaseWithTtl { generation, ttl_ms } => {
                                let grant = requested_grant_with_ttl(
                                    request.decode_payload().unwrap(),
                                    generation,
                                    ttl_ms,
                                );
                                *stored_lease.lock().unwrap() = Some(grant.lease.clone());
                                (success(&request, to_value(&grant).unwrap()), FollowUp::None)
                            }
                            Reply::LeaseThenPending(generation, applied) => {
                                let grant =
                                    requested_grant(request.decode_payload().unwrap(), generation);
                                *stored_lease.lock().unwrap() = Some(grant.lease);
                                applied.notify_one();
                                std::future::pending().await
                            }
                            Reply::LeaseThenDisconnect(generation) => {
                                let grant =
                                    requested_grant(request.decode_payload().unwrap(), generation);
                                *stored_lease.lock().unwrap() = Some(grant.lease);
                                return;
                            }
                            Reply::StoredLease => {
                                let lease = stored_lease.lock().unwrap().clone();
                                (success(&request, to_value(&lease).unwrap()), FollowUp::None)
                            }
                            Reply::Attachment {
                                initial,
                                events,
                                gate,
                                disconnect,
                                malformed_event,
                            } => (
                                success(&request, to_value(&initial).unwrap()),
                                FollowUp::Attachment {
                                    events,
                                    gate,
                                    disconnect,
                                    malformed: malformed_event,
                                },
                            ),
                            Reply::FragmentedAttachment { initial, event } => (
                                success(&request, to_value(&initial).unwrap()),
                                FollowUp::Fragmented(event),
                            ),
                        };
                        if peer.respond(&response).await.is_err() {
                            return;
                        }
                        if !matches!(follow_up.send(&mut peer, &transport).await, Ok(false)) {
                            return;
                        }
                    }
                });
            }
        });
        Self {
            _directory: directory,
            socket,
            requests,
            request_arrived,
            task,
        }
    }

    pub(crate) fn client(&self) -> Client {
        Client::new(&self.socket)
    }

    pub(crate) fn socket(&self) -> PathBuf {
        self.socket.clone()
    }

    pub(crate) fn methods(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.method.clone())
            .collect()
    }

    pub(crate) fn requests(&self) -> Vec<RequestEnvelope> {
        self.requests.lock().unwrap().clone()
    }
}

#[path = "support_wait.rs"]
mod wait;

impl Drop for FakeHost {
    fn drop(&mut self) {
        self.task.abort();
    }
}
