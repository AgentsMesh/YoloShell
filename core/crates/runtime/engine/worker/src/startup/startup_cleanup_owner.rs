use anyhow::{Context, Result, anyhow};
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::oneshot;

pub(crate) const OWNER_THREAD_NAME: &str = "yolo-startup-cleanup-owner";
pub(crate) const OWNER_SPAWN_CONTEXT: &str = "spawn startup cleanup settlement owner";
pub(crate) const OWNER_UNAVAILABLE_DETAIL: &str = "startup cleanup settlement owner is unavailable";
pub(crate) const OWNER_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(crate) type Settlement = std::result::Result<(), String>;

#[derive(Clone)]
pub(crate) struct StartupCleanupOwner {
    pub(crate) sender: Sender<SettlementBatch>,
}

pub(crate) struct SettlementBatch {
    handles: Vec<JoinHandle<()>>,
    completion: Option<oneshot::Sender<Settlement>>,
    failure: Option<String>,
}

pub(crate) static OWNER: OnceLock<std::result::Result<StartupCleanupOwner, String>> =
    OnceLock::new();

impl StartupCleanupOwner {
    pub(crate) fn capture() -> Result<Self> {
        OWNER
            .get_or_init(start_owner)
            .as_ref()
            .map(Clone::clone)
            .map_err(|detail| anyhow!(detail.clone()))
    }

    pub(crate) fn submit(
        &self,
        handles: Vec<JoinHandle<()>>,
    ) -> Result<oneshot::Receiver<Settlement>> {
        let (completion, receiver) = oneshot::channel();
        self.send(SettlementBatch::new(handles, Some(completion)))?;
        Ok(receiver)
    }

    pub(crate) fn detach(&self, handles: Vec<JoinHandle<()>>) {
        let _ = self.send(SettlementBatch::new(handles, None));
    }

    pub(crate) fn send(&self, batch: SettlementBatch) -> Result<()> {
        self.sender
            .send(batch)
            .map_err(|_| anyhow!(OWNER_UNAVAILABLE_DETAIL))
    }
}

impl SettlementBatch {
    pub(crate) fn new(
        handles: Vec<JoinHandle<()>>,
        completion: Option<oneshot::Sender<Settlement>>,
    ) -> Self {
        Self {
            handles,
            completion,
            failure: None,
        }
    }

    pub(crate) fn poll(&mut self) -> bool {
        let mut index = self.handles.len();
        while index > 0 {
            index -= 1;
            if !self.handles[index].is_finished() {
                continue;
            }
            let handle = self.handles.swap_remove(index);
            if let Err(payload) = handle.join()
                && self.failure.is_none()
            {
                self.failure = Some(super::supervision::panic_detail(payload));
            }
        }
        self.handles.is_empty()
    }

    pub(crate) fn complete(mut self) {
        let settlement = self.failure.take().map_or(Ok(()), Err);
        if let Some(completion) = self.completion.take() {
            let _ = completion.send(settlement);
        }
    }
}

pub(crate) fn detach(handles: Vec<JoinHandle<()>>) {
    match StartupCleanupOwner::capture() {
        Ok(owner) => owner.detach(handles),
        Err(error) => tracing::error!(%error, OWNER_UNAVAILABLE_DETAIL),
    }
}

pub(crate) fn start_owner() -> std::result::Result<StartupCleanupOwner, String> {
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name(OWNER_THREAD_NAME.to_owned())
        .spawn(move || run_owner(receiver))
        .context(OWNER_SPAWN_CONTEXT)
        .map_err(|error| error.to_string())?;
    Ok(StartupCleanupOwner { sender })
}

pub(crate) fn run_owner(receiver: Receiver<SettlementBatch>) {
    let mut active = Vec::new();
    loop {
        while let Ok(batch) = receiver.try_recv() {
            active.push(batch);
        }
        let mut index = active.len();
        while index > 0 {
            index -= 1;
            if active[index].poll() {
                active.swap_remove(index).complete();
            }
        }
        if active.is_empty() {
            let Ok(batch) = receiver.recv() else {
                return;
            };
            active.push(batch);
        } else {
            std::thread::sleep(OWNER_POLL_INTERVAL);
        }
    }
}
