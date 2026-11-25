use crate::{Federation, lifetime::OwnedGeneration};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use yolo_hub_dispatch::TerminalSource;
use yolo_hub_peer_protocol::{Call, Signed, TerminalUpdate};
use yolo_hub_peer_rpc::{RpcError, RpcFuture, Updates};

const EVENT_CAPACITY: usize = 1;

impl Federation {
    pub(crate) fn origin_stream(
        self: Arc<Self>,
        call: Signed<Call>,
        source: Box<dyn TerminalSource>,
    ) -> Result<Box<dyn Updates>> {
        let producer = OriginProducer {
            source,
            generation: OwnedGeneration::new(self)?,
        };
        let (sender, receiver) = mpsc::channel(EVENT_CAPACITY);
        let task = tokio::spawn(producer.run(call, sender));
        Ok(Box::new(OriginEvents { receiver, task }))
    }
}

// Keep the source in the same owner: its teardown precedes generation completion.
struct OriginProducer {
    source: Box<dyn TerminalSource>,
    generation: OwnedGeneration,
}

impl OriginProducer {
    async fn run(
        mut self,
        call: Signed<Call>,
        sender: mpsc::Sender<Result<Signed<TerminalUpdate>, RpcError>>,
    ) {
        let result = tokio::select! {
            _ = sender.closed() => Ok(()),
            result = self.generation.node().write_origin_events(&call, self.source.as_mut(), &sender) => result,
        };
        if result.is_err() {
            let _ = sender.send(Err(RpcError::Closed)).await;
        }
    }
}

struct OriginEvents {
    receiver: mpsc::Receiver<Result<Signed<TerminalUpdate>, RpcError>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for OriginEvents {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Updates for OriginEvents {
    fn next(&mut self) -> RpcFuture<'_, Signed<TerminalUpdate>> {
        Box::pin(async { self.receiver.recv().await.unwrap_or(Err(RpcError::Closed)) })
    }
}
