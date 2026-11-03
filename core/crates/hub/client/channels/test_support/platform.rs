use super::peer::deadline;
use tokio::{
    net::UnixListener,
    sync::{mpsc, oneshot},
};
use yolo_hub_events_protocol::{PlatformEventCursor, SubscribeEventsResult};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

/// One long-poll request for the test to answer.
pub struct Subscription {
    pub after: Option<PlatformEventCursor>,
    pub respond: oneshot::Sender<SubscribeEventsResult>,
}

/// Serve discovery and platform-event connections with one advertised method
/// list. `ListHubs` is answered locally; `SubscribeEvents` is forwarded to the
/// test. A connection that closes after Hello without requesting anything was a
/// watcher refusing a Hub without `SubscribeEvents`.
pub async fn serve(
    listener: UnixListener,
    hub: HubId,
    methods: Vec<Method>,
    subscriptions: mpsc::Sender<Subscription>,
    refusals: mpsc::Sender<()>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        let methods = methods.clone();
        let subscriptions = subscriptions.clone();
        let refusals = refusals.clone();
        tokio::spawn(async move {
            let mut peer = Framed::new(stream);
            let Ok(Incoming::Message(hello)) = peer.receive::<Call>(deadline()).await else {
                return;
            };
            if !matches!(hello.fields().command, Command::Hello {}) {
                return;
            }
            let reply = Reply::Hello {
                hello: Hello {
                    hub_id: hub,
                    version: HUB_PROTOCOL_VERSION,
                    methods,
                },
            };
            if peer
                .send(&Answer::new(&hello, reply).unwrap(), deadline())
                .await
                .is_err()
            {
                return;
            }
            let mut requested = false;
            loop {
                let Ok(Incoming::Message(request)) = peer.receive::<Call>(deadline()).await else {
                    if !requested {
                        let _ = refusals.send(()).await;
                    }
                    return;
                };
                requested = true;
                let reply = match &request.fields().command {
                    Command::ListHubs { .. } => Reply::Hubs {
                        hubs: Vec::new(),
                        next: None,
                    },
                    Command::SubscribeEvents { request } => {
                        let (respond, answer) = oneshot::channel();
                        if subscriptions
                            .send(Subscription {
                                after: request.fields().after,
                                respond,
                            })
                            .await
                            .is_err()
                        {
                            return;
                        }
                        let Ok(subscription) = answer.await else {
                            return;
                        };
                        Reply::EventSubscription { subscription }
                    }
                    _ => return,
                };
                if peer
                    .send(&Answer::new(&request, reply).unwrap(), deadline())
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
    }
}
