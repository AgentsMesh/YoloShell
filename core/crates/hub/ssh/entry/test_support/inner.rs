use super::*;
use std::sync::{Arc, atomic::Ordering};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    task::JoinSet,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, ClientStream};
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

#[derive(Clone, Copy)]
pub enum Service {
    Echo,
    DropMutation,
    StallBeforeHubAuth,
}

pub async fn run(
    io: ClientIo,
    key: Arc<russh::keys::PrivateKey>,
    registry: AccessRegistry,
    service: Service,
    counts: Arc<Counts>,
) {
    if matches!(service, Service::StallBeforeHubAuth) {
        std::future::pending::<()>().await;
    }
    let Ok((connection, mut incoming)) =
        yolo_hub_client_transport::accept(io, key, registry, deadline()).await
    else {
        return;
    };
    let hub = connection.admission().entry_hub();
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            _ = connection.closed() => break,
            channel = incoming.recv() => match channel {
                Some(channel) => {
                    let counts = counts.clone();
                    tasks.spawn(async move {
                        match service {
                            Service::Echo => echo(channel.stream).await,
                            Service::DropMutation => rpc(channel.stream, hub, counts).await,
                            Service::StallBeforeHubAuth => unreachable!(),
                        }
                    });
                }
                None => break,
            },
            Some(_) = tasks.join_next() => {},
        }
    }
    let _ = connection.close(deadline()).await;
}
async fn echo(mut stream: ClientStream) {
    let mut bytes = [u8::default(); ECHO_BUFFER];
    while let Ok(length) = stream.read(&mut bytes).await {
        if length == NO_COUNT || stream.write_all(&bytes[..length]).await.is_err() {
            break;
        }
    }
}
async fn rpc(stream: ClientStream, hub: yolo_hub_identity::HubId, counts: Arc<Counts>) {
    let mut framed = Framed::new(stream);
    let Ok(Incoming::Message(request)) = framed.receive::<Request>(deadline()).await else {
        return;
    };
    assert!(matches!(request.fields().command, Command::Hello {}));
    framed
        .send(
            &Response::new(
                &request,
                Reply::Hello {
                    hello: Hello {
                        hub_id: hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![Method::Hello, Method::Create],
                    },
                },
            )
            .unwrap(),
            deadline(),
        )
        .await
        .unwrap();
    if let Ok(Incoming::Message(request)) = framed.receive::<Request>(deadline()).await {
        assert!(matches!(request.fields().command, Command::Create { .. }));
        counts.mutations.fetch_add(COUNT_ONE, Ordering::SeqCst);
        // Deliberately lose the result after a real mutating request was received.
    }
}
