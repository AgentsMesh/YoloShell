mod broker_directory;
mod constants;
mod control;
mod diagnostics;
mod dispatch;
mod history;
mod inspection;
mod job;
mod route;
mod task_retention;
mod tasks;
use anyhow::Result;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::{sync::watch, task::JoinHandle};
use uuid::Uuid;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_remote_setup_protocol::RemoteSetupStatus;
use yolo_hub_ssh_profile_runtime::SshProfileRuntime;
use yolo_hub_state::HubState;
use yolo_paths::directory::PrivateDirectory;

pub struct RemoteSetupRuntime {
    engine: Arc<Engine>,
    jobs: Arc<Mutex<BTreeMap<Uuid, Job>>>,
}
struct Engine {
    state: Arc<HubState>,
    profiles: Arc<SshProfileRuntime>,
    inner: Arc<dyn Dispatcher>,
    root: PrivateDirectory,
}
struct Job {
    handle: JoinHandle<()>,
    status: watch::Receiver<RemoteSetupStatus>,
    admitted_at: tokio::time::Instant,
    completed_at: Option<tokio::time::Instant>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
impl RemoteSetupRuntime {
    pub fn open(
        state: Arc<HubState>,
        profiles: Arc<SshProfileRuntime>,
        inner: Arc<dyn Dispatcher>,
    ) -> Result<Arc<Self>> {
        anyhow::ensure!(state.hub_id() == inner.hub_id(), constants::INVALID);
        let parent = state.directory()?;
        let root = parent.ensure_descendant(
            &parent
                .path()
                .join(yolo_hub_remote_setup_store::REMOTE_SETUP_DIRECTORY),
        )?;
        Ok(Arc::new(Self {
            engine: Arc::new(Engine {
                state,
                profiles,
                inner,
                root,
            }),
            jobs: Arc::new(Mutex::new(BTreeMap::new())),
        }))
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
