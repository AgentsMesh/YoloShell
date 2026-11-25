use super::*;

/// The Host's change infrastructure: the catalogue notification feed and the
/// agent-event intake that feeds it one Worker subscription per live runtime.
pub(super) struct HostWatch {
    pub(super) feed: Arc<yolo_host_change_feed::ChangeFeed>,
    agents: AgentWatch,
}

impl HostWatch {
    pub(super) fn new() -> Self {
        Self {
            feed: Arc::new(yolo_host_change_feed::ChangeFeed::new()),
            agents: AgentWatch::new(),
        }
    }

    pub(super) fn agent_events(&self, record: RuntimeRecord) {
        self.agents.send(record);
    }
}

pub(super) struct AgentWatch {
    sender: tokio::sync::mpsc::UnboundedSender<RuntimeRecord>,
    receiver: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<RuntimeRecord>>>,
}

impl AgentWatch {
    pub(super) fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }

    pub(super) fn send(&self, record: RuntimeRecord) {
        let _ = self.sender.send(record);
    }

    fn take(&self) -> Option<tokio::sync::mpsc::UnboundedReceiver<RuntimeRecord>> {
        self.receiver.lock().ok()?.take()
    }
}

impl Host {
    /// Keep one Worker agent-event subscription per live runtime. The stream
    /// carries binding payloads only; the catalogue change it produces is a
    /// hint for subscribers to resynchronise. The connection setup borrows
    /// the Host, then the subscription drops it so a live Worker stream never
    /// keeps the Host or its socket alive.
    async fn watch_worker_agents(host: Arc<Self>, record: RuntimeRecord) {
        let feed = Arc::clone(&host.watch.feed);
        let terminal_id = record.terminal_id;
        let Ok((mut worker, _)) = host.custodied_worker_stream_with_hello(&record).await else {
            return;
        };
        let payload = yolo_runtime_protocol::WatchAgentEventsRequest {
            terminal_id,
            runtime_epoch: record.runtime_epoch,
        };
        let Ok(payload) = crate::protocol::to_value(&payload) else {
            return;
        };
        let request = RequestEnvelope::new(
            Uuid::new_v4().to_string(),
            method::WATCH_AGENT_EVENTS,
            payload,
        );
        drop(host);
        match exchange_worker_request(&mut worker, &request).await {
            // An older Worker without this stream is retried by the next
            // catalogue read; it must not itself trigger one.
            Ok(response) if response.ok => {}
            _ => return,
        }
        while worker.next_event().await.is_ok() {
            feed.touch(Some(terminal_id));
        }
        // The stream ended: the binding may have changed or the Worker left.
        feed.touch(Some(terminal_id));
    }
}

pub(super) fn spawn(host: &Arc<Host>) {
    let Some(mut records) = host.watch.agents.take() else {
        return;
    };
    let host = Arc::downgrade(host);
    tokio::spawn(async move {
        let watching = Arc::new(Mutex::new(std::collections::HashSet::new()));
        while let Some(record) = records.recv().await {
            let Some(host) = host.upgrade() else {
                break;
            };
            let key = (record.terminal_id, record.runtime_epoch, record.worker_id);
            if !lock_recover(&watching).insert(key) {
                continue;
            }
            let watching = Arc::clone(&watching);
            tokio::spawn(async move {
                Host::watch_worker_agents(host, record).await;
                lock_recover(&watching).remove(&key);
            });
        }
    });
}
