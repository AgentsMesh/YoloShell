use super::{channel_peer::*, constants::*, fixtures};
use crate::{
    Options, channel_network,
    constants::QUEUE_CAPACITY,
    network::{Job, Update},
};
use tokio::{net::UnixListener, sync::mpsc, task::JoinHandle, time::timeout_at};
use uuid::Uuid;
use yolo_hub_client::HubEndpoint;
use yolo_hub_client_channels::{ControlEvent, ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_hub_terminal_protocol::{InputFields, InputRequest, LeaseGrant};

pub struct Harness {
    _directory: tempfile::TempDir,
    task: JoinHandle<()>,
    pub jobs: mpsc::Sender<Job>,
    pub responses: mpsc::Receiver<Update>,
    pub peer: Peer,
    pub generation: Uuid,
    pub grant: LeaseGrant,
    pub scope: RequestScope,
    pub listener: UnixListener,
}

impl Harness {
    pub async fn start() -> Self {
        Self::with_grant(fixtures::grant(&fixtures::session())).await
    }

    pub async fn with_grant(grant: LeaseGrant) -> Self {
        let directory = tempfile::tempdir_in(NETWORK_TEST_ROOT).unwrap();
        let socket = directory.path().join(NETWORK_TEST_SOCKET);
        let listener = UnixListener::bind(&socket).unwrap();
        let hub = grant.fields().target.address.hub_id;
        let (jobs, incoming) = mpsc::channel(QUEUE_CAPACITY);
        let (updates, mut responses) = mpsc::channel(QUEUE_CAPACITY);
        let task = tokio::spawn(channel_network::network(
            Options {
                endpoint: HubEndpoint::local(socket),
                hub,
                layout: None,
            },
            incoming,
            updates,
        ));
        let (stream, _) = timeout_at(deadline(), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut peer = yolo_hub_jsonrpc_transport::Framed::new(stream);
        let greeting = receive(&mut peer).await;
        assert!(matches!(greeting.fields().command, Command::Hello {}));
        respond(&mut peer, &greeting, hello_reply(hub)).await;
        let Some(Update::Control(ControlEvent::Ready { generation, .. })) = responses.recv().await
        else {
            panic!()
        };
        Self {
            _directory: directory,
            task,
            jobs,
            responses,
            peer,
            generation,
            grant,
            scope: RequestScope::ordered_inputs(),
            listener,
        }
    }

    pub async fn send_hello(&self) {
        self.jobs
            .send(Job::Call {
                generation: Some(self.generation),
                origin: self.grant.fields().target.address.hub_id,
                command: Command::Hello {},
            })
            .await
            .ok()
            .unwrap();
    }

    pub async fn send_input(&self, data: Vec<u8>) -> ControlRequest {
        let request = ControlRequest {
            generation: Some(self.generation),
            destination: self.grant.fields().target.address.hub_id,
            command: Command::Input {
                request: InputRequest::new(InputFields {
                    request_id: Uuid::new_v4(),
                    grant: self.grant,
                    data,
                })
                .unwrap(),
            },
            validity: Some(self.scope.validity_until(
                std::time::Instant::now()
                    + std::time::Duration::from_millis(self.grant.fields().ttl_ms),
            )),
        };
        self.jobs
            .send(Job::UserInput(request.clone()))
            .await
            .ok()
            .unwrap();
        request
    }

    pub async fn event(&mut self) -> ControlEvent {
        let Some(Update::Control(event)) =
            timeout_at(deadline(), self.responses.recv()).await.unwrap()
        else {
            panic!()
        };
        event
    }

    pub async fn dispatch_barrier(&self) {
        // Holding all outer permits proves the adapter consumed every job,
        // including the last Input waiting at its full inner control queue.
        let permits = timeout_at(deadline(), self.jobs.reserve_many(QUEUE_CAPACITY))
            .await
            .unwrap()
            .unwrap();
        drop(permits);
    }
}

pub fn hello_reply(hub: yolo_hub_identity::HubId) -> Reply {
    Reply::Hello {
        hello: Hello {
            hub_id: hub,
            version: HUB_PROTOCOL_VERSION,
            methods: yolo_hub_rpc_protocol::SUPPORTED_METHODS
                .iter()
                .copied()
                .filter(|method| *method != Method::ListHubs)
                .collect(),
        },
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.task.abort();
    }
}
