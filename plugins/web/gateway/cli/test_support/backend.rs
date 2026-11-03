use super::{configuration::Configuration, constants::*};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{net::UnixListener, sync::Notify, task::JoinHandle};
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, FIRST_ACCESS_REVISION, OriginAccess,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, accept};
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

pub struct Backend {
    task: JoinHandle<()>,
    pub closed: Arc<Notify>,
}

impl Backend {
    pub async fn start(configuration: &Configuration) -> Self {
        let listener = UnixListener::bind(&configuration.socket).unwrap();
        let hub = configuration.hub.hub_id;
        let hub_key = configuration.hub_key.clone();
        let now = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let registry = AccessRegistry::new(AccessConfiguration {
            schema_version: ACCESS_SCHEMA_VERSION,
            entry_hub: hub,
            revision: FIRST_ACCESS_REVISION,
            clients: vec![ClientGrant {
                identity: configuration.identity,
                origins: vec![OriginAccess {
                    origin: hub,
                    rights: [Right::Observe].into_iter().collect(),
                }],
                expires_at_ms: now + GRANT_LIFETIME_MS,
            }],
        })
        .unwrap();
        let closed = Arc::new(Notify::new());
        let finished = closed.clone();
        let task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (connection, mut incoming) = accept(
                ClientIo::from_unix(socket).unwrap(),
                hub_key,
                registry.clone(),
                deadline(),
            )
            .await
            .unwrap();
            let channel = incoming.recv().await.unwrap();
            let mut channel = Framed::new(channel.stream);
            let Incoming::Message(call) = channel.receive::<Call>(deadline()).await.unwrap() else {
                panic!("missing hello");
            };
            assert_eq!(call.fields().command, Command::Hello {});
            let answer = Answer::new(
                &call,
                Reply::Hello {
                    hello: Hello {
                        hub_id: hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![Method::Hello],
                    },
                },
            )
            .unwrap();
            channel.send(&answer, deadline()).await.unwrap();
            connection.closed().await;
            finished.notify_one();
        });
        Self { task, closed }
    }
}

impl Drop for Backend {
    fn drop(&mut self) {
        self.task.abort();
    }
}
